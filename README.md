# dhl

Git worktree workspace manager. Create named workspaces that group worktrees from multiple repositories under `~/.dhl/<name>/`.

## Install

### macOS — Homebrew

```sh
brew tap mtib/tap
brew install dhl
```

### Cargo

```sh
cargo install --git https://github.com/mtib/dhl
```

### Pre-built binary

Download the latest binary for your platform from the [releases page](https://github.com/mtib/dhl/releases/tag/latest), extract it, and place `dhl` on your `PATH`.

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `dhl-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `dhl-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `dhl-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `dhl-aarch64-unknown-linux-gnu.tar.gz` |
| Windows x86_64 | `dhl-x86_64-pc-windows-msvc.zip` |

## Shell integration

Add this to your shell rc file to enable automatic `cd` and tab completion:

**zsh** (`~/.zshrc`)
```sh
eval "$(dhl shell-init)"
```

**bash** (`~/.bashrc`)
```sh
eval "$(dhl shell-init bash)"
```

**fish** (`~/.config/fish/config.fish`)
```fish
dhl shell-init fish | source
```

Shell integration enables:
- `dhl create` automatically cds into the new workspace
- `dhl goto <name>` navigates to an existing workspace
- Tab completion for all commands, workspace names, repo names, and flags

## Usage

### 1. Register repository roots

Tell dhl where your git repositories live:

```sh
dhl root add ~/Code
dhl root add ~/Projects
```

### 2. Browse available repositories

```sh
dhl repo list        # or: dhl repo ls
```

Repositories with the same name across multiple roots are shown with their full root path as a prefix (e.g. `/Users/you/Code/myrepo`). That prefixed form is accepted everywhere a repo name is expected.

### 3. Clone a repository into a root

```sh
dhl repo add https://github.com/org/myrepo
dhl repo add https://github.com/org/myrepo --root ~/Projects --name local-name
```

### 4. Create a workspace

```sh
# Name auto-generated from themed word list (e.g. "cargo-neural-branch")
dhl create api frontend

# Custom name
dhl create --name myfeature api frontend

# Branch specs
dhl create api:main:feat/x frontend:main:feat/x   # new branch from base
dhl create api::feat/x                             # new branch from HEAD

# Stay in current directory
dhl create --no-follow api frontend
```

Repo spec syntax:

| Spec | `git worktree add` equivalent |
|------|-------------------------------|
| `repo` | `git worktree add <path>` |
| `repo:from:to` | `git worktree add <path> -b <to> <from>` |
| `repo::to` | `git worktree add -b <to> <path>` |

### 5. Navigate to a workspace

```sh
dhl goto myfeature   # or: dhl go myfeature
```

With shell integration active this cds your shell into the workspace. Without it, prints a `DHL_CD:` marker line (useful for scripting).

```sh
# Print path only (no cd)
dhl get myfeature    # or: dhl path myfeature
```

### 6. List workspaces

```sh
dhl list
dhl ls
```

### 7. Delete a workspace

Removes all worktrees and the workspace directory:

```sh
dhl delete myfeature
dhl rm myfeature
```

### 8. Delete a repository from disk

```sh
dhl repo delete myrepo
dhl repo rm myrepo
```

## Command reference

```
dhl root add <path>               Register a repository root
dhl root remove <path>            Unregister a root
dhl root list / ls                List registered roots

dhl repo list / ls                List all repositories across roots
dhl repo add <url>                Clone a repository into a root
      [--root <path>]               Target root (required if >1 configured)
      [--name <name>]               Override local directory name
dhl repo delete / rm <name>       Remove a repository from disk

dhl create [--name <n>]           Create a workspace (cds in by default)
           [--no-follow]            Don't cd into the new workspace
           <repo>...
dhl goto   / go   <name>          Navigate shell to an existing workspace
dhl get    / path <name>          Print workspace path (no cd)
dhl list   / ls                   List all workspaces
dhl delete / rm   <name>          Delete a workspace and its worktrees

dhl shell-init [bash|zsh|fish]    Print shell integration code
```

## Tab completion

Completions are dynamic — they read live from the store:

| Context | Completes |
|---------|-----------|
| `dhl <TAB>` | all subcommands |
| `dhl create <TAB>` | repo names, `--name`, `--no-follow` |
| `dhl goto <TAB>` | workspace names |
| `dhl delete <TAB>` | workspace names |
| `dhl get <TAB>` | workspace names |
| `dhl root remove <TAB>` | registered roots |
| `dhl repo delete <TAB>` | repo names |
| `dhl repo add --root <TAB>` | registered roots |
| `dhl shell-init <TAB>` | `bash`, `zsh`, `fish` |

## How it works

- Workspace metadata is stored in a [RocksDB](https://rocksdb.org/) database at `~/.dhl/db/`.
- Each workspace is a directory under `~/.dhl/` containing git worktrees.
- Worktrees are created and removed via `git worktree add/remove`.
- Workspace names default to three words drawn from a package/delivery/code/AI vocabulary (e.g. `cargo-neural-branch`).
- Shell navigation works via a wrapper function emitted by `dhl shell-init` that intercepts the `DHL_CD:` marker printed by `create` and `goto`, strips it from displayed output, and cds to the path.

## Brew tap setup (for contributors)

The Homebrew formula lives in [mtib/homebrew-tap](https://github.com/mtib/homebrew-tap) and is updated automatically by CI on every push to `main`. To replicate this setup:

1. Create a GitHub repo named `homebrew-tap`.
2. Create `Formula/dhl.rb` (see the workflow for the template).
3. Add a `TAP_TOKEN` secret to the `dhl` repo — a fine-grained PAT with `Contents: Read and write` on the tap repo.
