use crate::{db::Store, repo::list_repos};

const TOP_LEVEL: &[&str] = &[
    "root", "repo", "create", "list", "ls", "delete", "rm", "get", "path", "goto", "go",
    "shell-init",
];
const ROOT_SUBCOMMANDS: &[&str] = &["add", "remove", "list", "ls"];
const REPO_SUBCOMMANDS: &[&str] = &["list", "ls", "add", "delete", "rm"];
const SHELLS: &[&str] = &["bash", "zsh", "fish"];

/// Given the tokenized command line (including "dhl" as words[0]),
/// return all valid completions for the current position.
/// The shell handles prefix filtering against the partial word.
pub fn completions(store: &Store, words: &[String]) -> Vec<String> {
    // words[0] = "dhl"
    // words[last] = current partial (may be empty)
    // words[last-1] = previous complete word
    let sub = words.get(1).map(String::as_str).unwrap_or("");
    let prev = words
        .len()
        .checked_sub(2)
        .and_then(|i| words.get(i))
        .map(String::as_str)
        .unwrap_or("");

    match sub {
        "" => strs(TOP_LEVEL),

        "root" => {
            let subsub = words.get(2).map(String::as_str).unwrap_or("");
            match subsub {
                "" => strs(ROOT_SUBCOMMANDS),
                "remove" => store.list_roots().unwrap_or_default(),
                _ => vec![],
            }
        }

        "repo" => {
            let subsub = words.get(2).map(String::as_str).unwrap_or("");
            match subsub {
                "" => strs(REPO_SUBCOMMANDS),
                "delete" | "rm" => repo_names(store),
                "add" => {
                    if prev == "--root" {
                        store.list_roots().unwrap_or_default()
                    } else {
                        // Only suggest flags not already present
                        let mut flags = vec![];
                        if !words.iter().any(|w| w == "--root") {
                            flags.push("--root".into());
                        }
                        if !words.iter().any(|w| w == "--name") {
                            flags.push("--name".into());
                        }
                        flags
                    }
                }
                _ => vec![],
            }
        }

        "create" => {
            if prev == "--name" {
                return vec![]; // free-form string value
            }
            let mut out = repo_names(store);
            if !words.iter().any(|w| w == "--no-follow") {
                out.push("--no-follow".into());
            }
            if !words.iter().any(|w| w == "--name") {
                out.push("--name".into());
            }
            out
        }

        "delete" | "rm" => {
            let mut out = workspace_names(store);
            if !words.iter().any(|w| w == "--all") {
                out.push("--all".into());
            }
            out
        }
        "get" | "path" => workspace_names(store),
        "goto" | "go" => workspace_names(store),
        "shell-init" => strs(SHELLS),

        _ => vec![],
    }
}

fn strs(s: &[&str]) -> Vec<String> {
    s.iter().map(|s| s.to_string()).collect()
}

fn repo_names(store: &Store) -> Vec<String> {
    list_repos(store)
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.display_name)
        .collect()
}

fn workspace_names(store: &Store) -> Vec<String> {
    store
        .list_workspaces()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}
