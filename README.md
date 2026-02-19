# web

Open URL aliases in a browser from the terminal.

## Install

```sh
cargo install --path .
```

## Usage

```sh
# Register an alias (comma-separated names for multiple)
web add gh https://github.com
web add claude,c https://claude.ai

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
```

## Shell Completion

### Dynamic (recommended)

Add to your `~/.zshrc` (or equivalent):

```sh
source <(COMPLETE=zsh web)
```

Replace `zsh` with `bash`, `fish`, or `elvish` as needed.

Dynamic completions stay in sync with your config automatically — alias names are completed as you type.

### Static

Generate a completion script and place it in your shell's `site-functions` directory:

```sh
# zsh
web completions zsh > ~/.zfunc/_web

# bash
web completions bash > /etc/bash_completion.d/web

# fish
web completions fish > ~/.config/fish/completions/web.fish
```

Static completions cover subcommands and flags but do not complete alias names.

## Config

Aliases are stored in `~/.config/website_opener/config.toml`:

```toml
[aliases]
gh = "https://github.com"
claude = "https://claude.ai"
c = "https://claude.ai"
```
