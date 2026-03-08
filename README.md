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

Add this to your `~/.zshrc`, `~/.bashrc`, or `~/.config/fish/config.fish` to enable automatic `cd` into newly created workspaces:

**zsh / bash**
```sh
eval "$(dhl shell-init)"        # defaults to zsh
eval "$(dhl shell-init bash)"
```

**fish**
```fish
dhl shell-init fish | source
```

With shell integration active, `dhl create` will `cd` you into the new workspace automatically. Pass `--no-follow` to stay in the current directory:

```sh
dhl create --no-follow api frontend
```

## Usage

### 1. Register repository roots

Tell dhl where your git repositories live:

```sh
dhl root add ~/Code
dhl root add ~/Projects
```

### 2. Browse repositories

```sh
dhl repo list          # or: dhl repo ls
```

Repositories with the same name in multiple roots are shown with their full root path prefix (e.g. `/Users/you/Code/myrepo`). That prefixed form can be used anywhere a repo name is accepted.

### 3. Clone a repository

```sh
dhl repo add https://github.com/org/myrepo
dhl repo add https://github.com/org/myrepo --root ~/Projects --name local-name
```

### 4. Create a workspace

```sh
# Name is auto-generated (three memorable words, e.g. "cargo-neural-branch")
dhl create api frontend

# With a custom name
dhl create --name myfeature api frontend

# Branch specs: repo:from:to  or  repo::new-branch
dhl create api:main:feat/x frontend:main:feat/x

# Stay in current directory instead of following the new workspace
dhl create --no-follow api frontend
```

Repo spec syntax:

| Spec | `git worktree add` equivalent |
|------|-------------------------------|
| `repo` | `git worktree add <path>` |
| `repo:from:to` | `git worktree add <path> -b <to> <from>` |
| `repo::to` | `git worktree add -b <to> <path>` |

The workspace lands at `~/.dhl/<name>/` with each repo as a subdirectory.

### 5. Navigate to a workspace

```sh
# With shell integration (automatic after dhl create):
dhl create api

# Manually:
cd $(dhl get myfeature)
cd $(dhl path myfeature)   # alias
```

### 6. List workspaces

```sh
dhl list
dhl ls
```

### 7. Delete a workspace

Removes worktrees and the workspace directory:

```sh
dhl delete myfeature
dhl rm myfeature
```

### 8. Delete a repository

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
      [--root <path>]               Target root (required if >1 root)
      [--name <name>]               Override local directory name
dhl repo delete / rm <name>       Remove a repository from disk

dhl create [--name <n>]           Create a workspace
           [--no-follow]            Don't cd into the new workspace
           <repo>...
dhl list   / ls                   List all workspaces
dhl delete / rm  <name>           Delete a workspace and its worktrees
dhl get    / path <name>          Print workspace path

dhl shell-init [bash|zsh|fish]    Print shell integration code
```

## How it works

- Workspace metadata is stored in a [RocksDB](https://rocksdb.org/) database at `~/.dhl/db/`.
- Each workspace is a directory under `~/.dhl/` containing git worktrees.
- Worktrees are created and removed via `git worktree add/remove`.
- Workspace names default to three words drawn from a package/delivery/code/AI vocabulary (e.g. `cargo-neural-branch`).
- Shell follow behaviour works via a wrapper function emitted by `dhl shell-init` that intercepts the `DHL_CD:` marker line printed by `dhl create`.

## Brew tap setup (for contributors)

The Homebrew formula lives in [mtib/homebrew-tap](https://github.com/mtib/homebrew-tap) and is updated automatically by CI on every push to `main`. To replicate this setup:

1. Create a GitHub repo named `homebrew-tap`.
2. Create `Formula/dhl.rb` (see the workflow for the template).
3. Add a `TAP_TOKEN` secret to the `dhl` repo — a fine-grained PAT with `Contents: Read and write` on the tap repo.
