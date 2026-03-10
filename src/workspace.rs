use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    db::Store,
    dhl_home,
    repo::{find_repo, parse_repo_spec},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub repos: Vec<WorkspaceRepo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceRepo {
    pub repo: String,
    pub worktree_path: PathBuf,
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<PathBuf>,
}

impl Workspace {
    pub fn create(store: &Store, name: String, repo_specs: &[String]) -> Result<Self> {
        let home = dhl_home()?;
        let workspace_path = home.join(&name);
        std::fs::create_dir_all(&workspace_path)?;

        let mut repos = Vec::new();
        for spec in repo_specs {
            let (repo_name, from_branch, to_branch) = parse_repo_spec(spec);
            let repo_path = find_repo(store, &repo_name)?;
            let worktree_path = workspace_path.join(&repo_name);

            create_worktree(
                &repo_path,
                &worktree_path,
                from_branch.as_deref(),
                to_branch.as_deref(),
            )?;

            copy_env_files(&repo_path, &worktree_path);

            repos.push(WorkspaceRepo {
                repo: repo_name,
                worktree_path,
                branch: to_branch.or(from_branch.clone()),
                base_branch: from_branch,
                source_path: Some(repo_path),
            });
        }

        let ws = Workspace {
            name: name.clone(),
            path: workspace_path,
            repos,
        };

        write_claude_md(&ws);

        let serialized = serde_json::to_string(&ws)?;
        store.put_workspace(&name, &serialized)?;
        Ok(ws)
    }

    pub fn load(store: &Store, name: &str) -> Result<Option<Self>> {
        match store.get_workspace(name)? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub fn list_all(store: &Store) -> Result<Vec<Self>> {
        let entries = store.list_workspaces()?;
        let mut workspaces = Vec::new();
        for (_, json) in entries {
            workspaces.push(serde_json::from_str(&json)?);
        }
        Ok(workspaces)
    }

    pub fn delete(store: &Store, name: &str) -> Result<()> {
        let ws = Self::load(store, name)?
            .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", name))?;

        // Remove each worktree
        for repo_entry in &ws.repos {
            let repo_path =
                find_repo_path_from_worktree(store, &repo_entry.repo, &repo_entry.worktree_path);
            if let Some(repo_path) = repo_path {
                remove_worktree(&repo_path, &repo_entry.worktree_path);
            }
        }

        // Remove workspace directory
        if ws.path.exists() {
            std::fs::remove_dir_all(&ws.path)?;
        }

        store.delete_workspace(name)?;
        Ok(())
    }
}

fn create_worktree(
    repo: &Path,
    worktree_path: &Path,
    from_branch: Option<&str>,
    to_branch: Option<&str>,
) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).arg("worktree").arg("add");

    match (from_branch, to_branch) {
        (Some(from), Some(to)) => {
            cmd.arg(worktree_path).arg("-b").arg(to).arg(from);
        }
        (None, Some(to)) => {
            cmd.arg("-b").arg(to).arg(worktree_path);
        }
        (Some(from), None) => {
            cmd.arg(worktree_path).arg(from);
        }
        (None, None) => {
            cmd.arg(worktree_path);
        }
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree add failed: {}", stderr);
    }
    Ok(())
}

fn find_repo_path_from_worktree(
    store: &Store,
    repo_name: &str,
    _worktree: &Path,
) -> Option<PathBuf> {
    crate::repo::find_repo(store, repo_name).ok()
}

fn remove_worktree(repo: &Path, worktree_path: &Path) {
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(worktree_path)
        .output();
}

/// Copy gitignored `*.env*` files from the source repo into the new worktree,
/// preserving their relative paths.
fn copy_env_files(source: &Path, worktree: &Path) {
    // List ignored files that are present on disk.
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .arg("ls-files")
        .arg("--others")
        .arg("--ignored")
        .arg("--exclude-standard")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for rel in stdout.lines() {
        // Match any path component containing ".env" (e.g. .env, .env.local, config/.env.dev)
        let filename = match Path::new(rel).file_name().and_then(|f| f.to_str()) {
            Some(f) => f,
            None => continue,
        };
        if !filename.contains(".env") {
            continue;
        }

        let src_file = source.join(rel);
        let dst_file = worktree.join(rel);

        if !src_file.is_file() {
            continue;
        }

        if let Some(parent) = dst_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::copy(&src_file, &dst_file).is_ok() {
            println!("  Copied {}", rel);
        }
    }
}

fn write_claude_md(ws: &Workspace) {
    use std::fmt::Write;

    let mut content = String::new();
    let _ = writeln!(content, "# Workspace: {}\n", ws.name);
    let _ = writeln!(
        content,
        "This is a [dhl](https://github.com/mtib/dhl) workspace using git worktrees. \
         Each subdirectory is a worktree checked out from its parent repository.\n"
    );
    let _ = writeln!(
        content,
        "All work should be contained to this workspace. \
         Do not read from or modify the source repositories directly. \
         If you need code from a repository that is not part of this workspace \
         but is referenced by a source path above, propose adding a worktree for it \
         using `git -C <source_path> worktree add {}/<name>`. \
         Before doing so, verify the source repository's current branch and \
         that the working tree is clean (`git -C <source_path> status`). \
         If you need a completely new set of repositories, \
         propose creating a new workspace with `dhl create` instead.\n",
        ws.path.display()
    );

    let _ = writeln!(
        content,
        "Because workspaces are created on demand, any Docker Compose setups \
         will likely need to be started before use. \
         Shut them down when work is completed. \
         Use Docker Compose overrides to avoid port conflicts with other workspaces, \
         and prefer not exposing ports at all when not absolutely necessary.\n"
    );

    let _ = writeln!(content, "## Repositories\n");
    let _ = writeln!(
        content,
        "| Repo | Branch | Based on | Source (reference only) |"
    );
    let _ = writeln!(
        content,
        "|------|--------|----------|------------------------|"
    );

    for repo in &ws.repos {
        let branch = repo.branch.as_deref().unwrap_or("(default)");
        let base = repo.base_branch.as_deref().unwrap_or("-");
        let source = repo
            .source_path
            .as_ref()
            .map(|p| format!("`{}`", p.display()))
            .unwrap_or_else(|| "-".to_string());
        let _ = writeln!(
            content,
            "| `{}` | `{}` | {} | {} |",
            repo.repo, branch, base, source
        );
    }

    let claude_md_path = ws.path.join("CLAUDE.md");
    let _ = std::fs::write(&claude_md_path, content);
}
