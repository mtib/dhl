use anyhow::Result;
use clap::{Parser, Subcommand};
use dhl::{
    db::Store,
    dhl_home,
    repo::{clone_repo, delete_repo, list_repos},
    workspace::Workspace,
};
use uuid::Uuid;

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
        /// Name for the workspace (default: random UUID)
        #[arg(long, short)]
        name: Option<String>,
        /// Repo specs: name, name:from:to, or name::to
        repos: Vec<String>,
    },
    /// List all workspaces
    #[command(alias = "ls")]
    List,
    /// Delete a workspace and its worktrees
    #[command(alias = "rm")]
    Delete {
        /// Workspace name
        name: String,
    },
    /// Get the path of a workspace
    #[command(alias = "path")]
    Get {
        /// Workspace name
        name: String,
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

        Commands::Create { name, repos } => {
            if repos.is_empty() {
                anyhow::bail!("Specify at least one repo.");
            }
            let name = name.unwrap_or_else(|| Uuid::new_v4().to_string());
            let ws = Workspace::create(&store, name, &repos)?;
            println!("Created workspace '{}' at {}", ws.name, ws.path.display());
            for r in &ws.repos {
                println!("  {} -> {}", r.repo, r.worktree_path.display());
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

        Commands::Delete { name } => {
            Workspace::delete(&store, &name)?;
            println!("Deleted workspace '{}'.", name);
        }

        Commands::Get { name } => {
            match Workspace::load(&store, &name)? {
                Some(ws) => println!("{}", ws.path.display()),
                None => anyhow::bail!("Workspace '{}' not found.", name),
            }
        }
    }

    Ok(())
}
