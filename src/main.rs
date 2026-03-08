use anyhow::Result;
use clap::{Parser, Subcommand};
use dhl::{
    complete,
    db::Store,
    dhl_home,
    names::random_name,
    repo::{clone_repo, delete_repo, list_repos},
    workspace::Workspace,
};

/// Marker prefix written to stdout so the shell wrapper can cd to the new workspace.
pub const FOLLOW_MARKER: &str = "DHL_CD:";

#[derive(Parser)]
#[command(name = "dhl", about = "Git worktree workspace manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage repository root directories
    Root {
        #[command(subcommand)]
        action: RootAction,
    },
    /// Manage repositories within roots
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },
    /// Create a new workspace with worktrees for the given repos
    Create {
        /// Name for the workspace (default: three random words)
        #[arg(long, short)]
        name: Option<String>,
        /// Do not cd into the workspace after creation
        #[arg(long)]
        no_follow: bool,
        /// Repo specs: name, name:from:to, or name::to
        repos: Vec<String>,
    },
    /// List all workspaces
    #[command(alias = "ls")]
    List,
    /// Delete one or more workspaces and their worktrees
    #[command(alias = "rm")]
    Delete {
        /// Workspace names to delete
        names: Vec<String>,
        /// Delete all workspaces
        #[arg(long)]
        all: bool,
    },
    /// Get the path of a workspace
    #[command(alias = "path")]
    Get {
        /// Workspace name
        name: String,
    },
    /// Navigate the shell to a workspace (requires shell integration)
    #[command(alias = "go")]
    Goto {
        /// Workspace name
        name: String,
    },
    /// Print shell integration code; add `eval "$(dhl shell-init)"` to your shell rc
    ShellInit {
        /// Shell type (bash, zsh, fish). Defaults to zsh.
        #[arg(default_value = "zsh")]
        shell: String,
    },
    /// Dynamic completion helper (called by shell completion functions)
    #[command(hide = true, name = "__complete")]
    Complete {
        /// Full tokenized command line, including "dhl" as the first token
        #[arg(raw = true)]
        words: Vec<String>,
    },
}

#[derive(Subcommand)]
enum RootAction {
    /// Add a repository root directory
    Add { path: String },
    /// Remove a repository root directory
    Remove { path: String },
    /// List all repository root directories
    #[command(alias = "ls")]
    List,
}

#[derive(Subcommand)]
enum RepoAction {
    /// List all repositories across all roots
    #[command(alias = "ls")]
    List,
    /// Clone a git repository into a root
    Add {
        /// Git URL to clone
        url: String,
        /// Root directory to clone into (required when multiple roots are configured)
        #[arg(long)]
        root: Option<String>,
        /// Local directory name (defaults to repo name derived from URL)
        #[arg(long)]
        name: Option<String>,
    },
    /// Delete a repository from disk
    #[command(alias = "rm")]
    Delete {
        /// Repository name (or root-prefixed name for disambiguation)
        name: String,
    },
}

fn open_store() -> Result<Store> {
    let home = dhl_home()?;
    Store::open(home.join("db"))
}

fn shell_init(shell: &str) -> &'static str {
    match shell {
        "bash" => concat!(
            // follow wrapper
            "dhl() {\n",
            "  if [[ \"$1\" == create || \"$1\" == goto || \"$1\" == go ]] && [[ \" $* \" != *\" --no-follow \"* ]]; then\n",
            "    local _out _status _cd\n",
            "    _out=$(command dhl \"$@\")\n",
            "    _status=$?\n",
            "    _cd=$(printf '%s\\n' \"$_out\" | grep '^DHL_CD:' | head -1 | cut -c8-)\n",
            "    printf '%s\\n' \"$_out\" | grep -v '^DHL_CD:'\n",
            "    [[ -n \"$_cd\" ]] && cd \"$_cd\"\n",
            "    return \"$_status\"\n",
            "  else\n",
            "    command dhl \"$@\"\n",
            "  fi\n",
            "}\n",
            // completion
            "_dhl_complete() {\n",
            "  local IFS=$'\\n'\n",
            "  COMPREPLY=($(command dhl __complete -- \"${COMP_WORDS[@]}\"))\n",
            "}\n",
            "complete -F _dhl_complete dhl\n",
        ),
        "fish" => concat!(
            // follow wrapper
            "function dhl\n",
            "    if contains -- \"$argv[1]\" create goto go; and not contains -- --no-follow $argv\n",
            "        set _dhl_out (command dhl $argv)\n",
            "        set _dhl_status $status\n",
            "        for line in $_dhl_out\n",
            "            if string match -q 'DHL_CD:*' $line\n",
            "                cd (string replace 'DHL_CD:' '' $line)\n",
            "            else\n",
            "                echo $line\n",
            "            end\n",
            "        end\n",
            "        return $_dhl_status\n",
            "    else\n",
            "        command dhl $argv\n",
            "    end\n",
            "end\n",
            // completion — fish passes tokens as separate args via command substitution
            "complete -c dhl -f -a '(command dhl __complete -- (commandline -opc))'\n",
        ),
        // zsh (default)
        _ => concat!(
            // follow wrapper
            "dhl() {\n",
            "  if [[ \"$1\" == create || \"$1\" == goto || \"$1\" == go ]] && [[ \" $* \" != *\" --no-follow \"* ]]; then\n",
            "    local _out _status _cd\n",
            "    _out=$(command dhl \"$@\")\n",
            "    _status=$?\n",
            "    _cd=$(printf '%s\\n' \"$_out\" | grep '^DHL_CD:' | head -1 | cut -c8-)\n",
            "    printf '%s\\n' \"$_out\" | grep -v '^DHL_CD:'\n",
            "    [[ -n \"$_cd\" ]] && cd \"$_cd\"\n",
            "    return \"$_status\"\n",
            "  else\n",
            "    command dhl \"$@\"\n",
            "  fi\n",
            "}\n",
            // completion
            "_dhl_complete() {\n",
            "  local -a _comps\n",
            "  _comps=(${(f)\"$(command dhl __complete -- \"${words[@]}\")\"})\n",
            "  compadd -a _comps\n",
            "}\n",
            // compdef is only available after compinit; initialise if needed
            "if ! (( $+functions[compdef] )); then\n",
            "  autoload -Uz compinit && compinit\n",
            "fi\n",
            "compdef _dhl_complete dhl\n",
        ),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = open_store()?;

    match cli.command {
        Commands::Root { action } => match action {
            RootAction::Add { path } => {
                let canonical = std::fs::canonicalize(&path)
                    .unwrap_or_else(|_| std::path::PathBuf::from(&path));
                store.add_root(&canonical.to_string_lossy())?;
                println!("Added root: {}", canonical.display());
            }
            RootAction::Remove { path } => {
                let canonical = std::fs::canonicalize(&path)
                    .unwrap_or_else(|_| std::path::PathBuf::from(&path));
                store.remove_root(&canonical.to_string_lossy())?;
                println!("Removed root: {}", canonical.display());
            }
            RootAction::List => {
                let roots = store.list_roots()?;
                if roots.is_empty() {
                    println!("No roots configured.");
                } else {
                    for root in roots {
                        println!("{}", root);
                    }
                }
            }
        },

        Commands::Repo { action } => match action {
            RepoAction::List => {
                let repos = list_repos(&store)?;
                if repos.is_empty() {
                    println!("No repositories found.");
                } else {
                    for repo in repos {
                        println!("{}", repo.display_name);
                    }
                }
            }
            RepoAction::Add { url, root, name } => {
                clone_repo(&store, &url, root.as_deref(), name.as_deref())?;
            }
            RepoAction::Delete { name } => {
                delete_repo(&store, &name)?;
            }
        },

        Commands::Create { name, no_follow, repos } => {
            if repos.is_empty() {
                anyhow::bail!("Specify at least one repo.");
            }
            let name = name.unwrap_or_else(random_name);
            let ws = Workspace::create(&store, name, &repos)?;
            println!("Created workspace '{}' at {}", ws.name, ws.path.display());
            for r in &ws.repos {
                println!("  {} -> {}", r.repo, r.worktree_path.display());
            }
            if !no_follow {
                println!("{}{}", FOLLOW_MARKER, ws.path.display());
            }
        }

        Commands::List => {
            let workspaces = Workspace::list_all(&store)?;
            if workspaces.is_empty() {
                println!("No workspaces.");
            } else {
                for ws in workspaces {
                    println!("{}\t{}", ws.name, ws.path.display());
                }
            }
        }

        Commands::Delete { names, all } => {
            if !all && names.is_empty() {
                anyhow::bail!("Specify at least one workspace name or pass --all.");
            }
            let to_delete: Vec<String> = if all {
                Workspace::list_all(&store)?.into_iter().map(|ws| ws.name).collect()
            } else {
                names
            };
            for name in &to_delete {
                Workspace::delete(&store, name)?;
                println!("Deleted workspace '{}'.", name);
            }
        }

        Commands::Get { name } => {
            match Workspace::load(&store, &name)? {
                Some(ws) => println!("{}", ws.path.display()),
                None => anyhow::bail!("Workspace '{}' not found.", name),
            }
        }

        Commands::Goto { name } => {
            match Workspace::load(&store, &name)? {
                Some(ws) => println!("{}{}", FOLLOW_MARKER, ws.path.display()),
                None => anyhow::bail!("Workspace '{}' not found.", name),
            }
        }

        Commands::ShellInit { shell } => {
            print!("{}", shell_init(&shell));
        }

        Commands::Complete { words } => {
            for c in complete::completions(&store, &words) {
                println!("{}", c);
            }
        }
    }

    Ok(())
}
