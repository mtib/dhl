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

## Usage

### 1. Register repository roots

Tell dhl where your git repositories live:

```sh
dhl root add ~/Code
dhl root add ~/Projects
```

### 2. Create a workspace

```sh
# Worktrees for two repos, checked out at HEAD
dhl create myfeature api frontend

# With a custom branch from a base
dhl create myfeature api:main:feat/my-feature frontend:main:feat/my-feature

# Mix of specs; short form name::new-branch checks out a new branch from HEAD
dhl create myfeature api::feat/my-feature frontend

# Named workspace (otherwise a UUID is generated)
dhl create --name myfeature api frontend
```

Repo spec syntax:

| Spec | `git worktree add` equivalent |
|------|-------------------------------|
| `repo` | `git worktree add <path>` |
| `repo:from:to` | `git worktree add <path> -b <to> <from>` |
| `repo::to` | `git worktree add -b <to> <path>` |

The workspace is created under `~/.dhl/<name>/` with each repo as a subdirectory.

### 3. Navigate to a workspace

```sh
cd $(dhl get myfeature)
# or
cd $(dhl path myfeature)
```

### 4. List workspaces

```sh
dhl list
dhl ls
```

### 5. Delete a workspace

Removes worktrees and the workspace directory:

```sh
dhl delete myfeature
dhl rm myfeature
```

## Command reference

```
dhl root add <path>       Register a repository root directory
dhl root remove <path>    Unregister a root
dhl root list             List registered roots

dhl create [--name <n>] <repo>...   Create a workspace
dhl list   (ls)                     List all workspaces
dhl delete (rm) <name>              Delete a workspace
dhl get    (path) <name>            Print workspace path
```

## How it works

- Workspace metadata is stored in a [RocksDB](https://rocksdb.org/) database at `~/.dhl/db/`.
- Each workspace is a directory under `~/.dhl/` containing git worktrees.
- Worktrees are created and removed via `git worktree add/remove`.

## Brew tap setup (for contributors)

The Homebrew formula lives in [mtib/homebrew-tap](https://github.com/mtib/homebrew-tap). It is updated automatically by CI on every push to `main`. To set this up yourself:

1. Create a GitHub repo named `homebrew-tap` under your account.
2. Create `Formula/dhl.rb` (see the workflow for the template).
3. Add a `TAP_TOKEN` secret to the `dhl` repo — a GitHub PAT with `repo` scope on the tap repo.
