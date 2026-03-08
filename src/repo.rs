use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::db::Store;

pub struct RepoEntry {
    pub name: String,
    pub root: String,
    pub path: PathBuf,
    /// Short name when unique across roots, otherwise "<root>/<name>".
    pub display_name: String,
}

/// List all git repositories found across all roots, resolving display names.
/// Duplicate bare names get prefixed with their root path.
pub fn list_repos(store: &Store) -> Result<Vec<RepoEntry>> {
    let roots = store.list_roots()?;
    let mut entries: Vec<(String, String, PathBuf)> = Vec::new(); // (name, root, path)

    for root in &roots {
        let root_path = Path::new(root);
        let read_dir = match std::fs::read_dir(root_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if is_git_repo(&path) {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                entries.push((name, root.clone(), path));
            }
        }
    }

    // Count how many roots contain each bare name
    let name_count: std::collections::HashMap<String, usize> =
        entries.iter().fold(std::collections::HashMap::new(), |mut m, (name, _, _)| {
            *m.entry(name.clone()).or_insert(0) += 1;
            m
        });

    let result = entries
        .into_iter()
        .map(|(name, root, path)| {
            let display_name = if name_count[&name] > 1 {
                format!("{}/{}", root, name)
            } else {
                name.clone()
            };
            RepoEntry { name, root, path, display_name }
        })
        .collect();

    Ok(result)
}

/// Find a git repository by name or root-prefixed name across all stored roots.
/// Accepts:
///   - bare name:           "myrepo"
///   - root-prefixed name:  "/path/to/root/myrepo"
pub fn find_repo(store: &Store, name: &str) -> Result<PathBuf> {
    // If the name looks like an absolute path, validate directly.
    if name.starts_with('/') {
        let p = PathBuf::from(name);
        if is_git_repo(&p) {
            return Ok(p);
        }
        bail!("'{}' is not a git repository", name);
    }

    let roots = store.list_roots()?;
    if roots.is_empty() {
        bail!("No repository roots configured. Use `dhl root add <path>` first.");
    }

    // Check if name is "<root>/<bare_name>" for a known root.
    for root in &roots {
        let prefix = format!("{}/", root);
        if let Some(bare) = name.strip_prefix(&prefix) {
            let candidate = Path::new(root).join(bare);
            if is_git_repo(&candidate) {
                return Ok(candidate);
            }
        }
    }

    // Plain name: search all roots.
    let mut matches: Vec<PathBuf> = Vec::new();
    for root in &roots {
        let candidate = Path::new(root).join(name);
        if is_git_repo(&candidate) {
            matches.push(candidate);
        }
    }
    match matches.len() {
        0 => bail!("Repository '{}' not found in any root: {:?}", name, roots),
        1 => Ok(matches.remove(0)),
        _ => bail!(
            "Repository '{}' is ambiguous (found in multiple roots: {:?}). Use the full path.",
            name,
            matches
        ),
    }
}

/// Clone a git URL into a root directory, optionally with a custom local name.
/// Returns the path of the cloned repository.
pub fn clone_repo(store: &Store, url: &str, root: Option<&str>, local_name: Option<&str>) -> Result<PathBuf> {
    let roots = store.list_roots()?;
    if roots.is_empty() {
        bail!("No repository roots configured. Use `dhl root add <path>` first.");
    }

    let target_root = if let Some(r) = root {
        let canonical = std::fs::canonicalize(r)
            .unwrap_or_else(|_| PathBuf::from(r));
        let s = canonical.to_string_lossy().into_owned();
        if !roots.contains(&s) {
            bail!("'{}' is not a registered root. Add it first with `dhl root add`.", s);
        }
        s
    } else if roots.len() == 1 {
        roots[0].clone()
    } else {
        bail!("Multiple roots configured; specify which one with --root.");
    };

    let target_dir = if let Some(name) = local_name {
        Path::new(&target_root).join(name)
    } else {
        // Derive name from URL (last path segment, strip .git suffix).
        let raw = url.trim_end_matches('/');
        let segment = raw.rsplit('/').next().unwrap_or(raw);
        let name = segment.strip_suffix(".git").unwrap_or(segment);
        Path::new(&target_root).join(name)
    };

    let output = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(&target_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed: {}", stderr);
    }

    println!("Cloned '{}' into {}", url, target_dir.display());
    Ok(target_dir)
}

/// Remove a git repository from disk.
pub fn delete_repo(store: &Store, name: &str) -> Result<()> {
    let path = find_repo(store, name)?;
    std::fs::remove_dir_all(&path)?;
    println!("Deleted {}", path.display());
    Ok(())
}

fn is_git_repo(path: &Path) -> bool {
    path.is_dir() && path.join(".git").exists()
}

/// Parse a repo spec of the form `name`, `name:from:to`, or `name::to`.
/// The name portion may itself contain `/` (root-prefixed form).
/// Returns (repo_name, Option<from_branch>, Option<to_branch>).
pub fn parse_repo_spec(spec: &str) -> (String, Option<String>, Option<String>) {
    // Split on `:` but be careful: an absolute path like `/foo/bar` has no `:`,
    // and a prefixed name `/foo/bar/repo:from:to` needs the first `:` that isn't
    // part of a Windows drive letter. We find the first `:` that is followed by
    // a non-`/` character or is not the second character (drive letters are `X:`).
    // Simplest heuristic: find colons that separate branch specs.
    // Strategy: find up to 2 colons from the right (branch specs are after the last colons).
    let bytes = spec.as_bytes();
    let mut colon_positions: Vec<usize> = bytes
        .iter()
        .enumerate()
        .filter(|&(_, &b)| b == b':')
        .map(|(i, _)| i)
        .collect();

    match colon_positions.len() {
        0 => (spec.to_string(), None, None),
        1 => {
            // Only one colon: treat as name:from (no to-branch).
            let pos = colon_positions[0];
            let name = spec[..pos].to_string();
            let from = &spec[pos + 1..];
            let from = if from.is_empty() { None } else { Some(from.to_string()) };
            (name, from, None)
        }
        _ => {
            // Two or more colons: last two delimit from:to.
            let to_pos = colon_positions.pop().unwrap();
            let from_pos = colon_positions.pop().unwrap();
            let name = spec[..from_pos].to_string();
            let from = &spec[from_pos + 1..to_pos];
            let to = &spec[to_pos + 1..];
            let from = if from.is_empty() { None } else { Some(from.to_string()) };
            let to = if to.is_empty() { None } else { Some(to.to_string()) };
            (name, from, to)
        }
    }
}
