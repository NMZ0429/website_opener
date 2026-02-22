# web

Open URL aliases in a browser from the terminal.

## Install

Via the installer script (macOS):

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/NMZ0429/website_opener/releases/latest/download/website_opener-installer.sh | sh
```

Or build from source:

```sh
cargo install --path .
```

## Usage

```sh
# Register an alias (comma-separated names for multiple)
web add gh https://github.com
web add claude,c https://claude.ai

# URLs with special characters (?, &, etc.) must be quoted
web add aws 'https://myapps.microsoft.com/signin/myapp?tenantId=abc123'

# Open an alias
web gh

# Open in a specific browser
web --safari gh
web --chrome gh
web --firefox gh
web --brave gh

# List all aliases
web list

# Remove alias(es)
web remove gh
web remove claude,c

# Export all aliases to stdout (TOML format)
web export
web export > my-aliases.toml
```

## Shell Completion

Add to your `~/.zshrc` (or equivalent):

```sh
source <(COMPLETE=zsh web)
# or
source <(web completions zsh)
```

Replace `zsh` with `bash`, `fish`, or `elvish` as needed.

Alternatively, write the completion script to a file:

```sh
# zsh
web completions zsh > ~/.zfunc/_web

# bash
web completions bash > /etc/bash_completion.d/web

# fish
web completions fish > ~/.config/fish/completions/web.fish
```

Completions stay in sync with your config automatically — alias names are completed as you type.

## Config

Aliases are stored in `~/.config/web/config.toml`:

```toml
[aliases]
gh = "https://github.com"
claude = "https://claude.ai"
c = "https://claude.ai"
```

## Release

Releases are automated with [dist](https://opensource.axo.dev/cargo-dist/). Pushing a version tag triggers GitHub Actions to build macOS binaries and create a GitHub Release with installers.

```sh
# 1. Bump version in Cargo.toml
# 2. Commit
git add Cargo.toml Cargo.lock
git commit -m "release: v0.2.0"

# 3. Tag and push
git tag v0.2.0
git push origin main --tags
```

The workflow builds `aarch64-apple-darwin` and `x86_64-apple-darwin` binaries and generates a shell installer script.

To regenerate the release workflow after changing dist config:

```sh
dist generate
```
