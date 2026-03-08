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

            repos.push(WorkspaceRepo {
                repo: repo_name,
                worktree_path,
                branch: to_branch.or(from_branch),
            });
        }

        let ws = Workspace {
            name: name.clone(),
            path: workspace_path,
            repos,
        };
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
