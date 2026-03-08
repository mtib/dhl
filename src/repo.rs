use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::db::Store;

/// Find a git repository by name across all stored roots.
pub fn find_repo(store: &Store, name: &str) -> Result<PathBuf> {
    let roots = store.list_roots()?;
    if roots.is_empty() {
        bail!("No repository roots configured. Use `dhl root add <path>` first.");
    }
    for root in &roots {
        let candidate = Path::new(root).join(name);
        if candidate.join(".git").exists() || is_git_repo(&candidate) {
            return Ok(candidate);
        }
    }
    bail!("Repository '{}' not found in any root: {:?}", name, roots);
}

fn is_git_repo(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    // Also handles worktrees (no .git dir, just a .git file)
    path.join(".git").exists()
}

/// Parse a repo spec of the form `name`, `name:from:to`, or `name::to`.
/// Returns (repo_name, Option<from_branch>, Option<to_branch>).
pub fn parse_repo_spec(spec: &str) -> (String, Option<String>, Option<String>) {
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    match parts.as_slice() {
        [name] => (name.to_string(), None, None),
        [name, from, to] => {
            let from = if from.is_empty() { None } else { Some(from.to_string()) };
            let to = if to.is_empty() { None } else { Some(to.to_string()) };
            (name.to_string(), from, to)
        }
        _ => (spec.to_string(), None, None),
    }
}
