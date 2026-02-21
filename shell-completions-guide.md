# Shell Completions with clap: A Practical Guide

This document explains how dynamic shell completions were implemented for the `web` CLI using `clap` and `clap_complete`, including the problems we ran into and how we solved them.

## How Shell Completion Works (The Big Picture)

When you press `<TAB>` in your terminal, the **shell** (not your program) intercepts the keypress. The shell looks up a registered completion function for the current command, calls it, and displays the results.

There are two fundamentally different approaches:

### Static Completions

A script is generated once at build/install time via `clap_complete::generate()`. This script is pure shell code that hard-codes the subcommands, flags, and argument structure.

```
[build time] binary generates shell script → saved to file
[runtime]    shell reads the script → offers completions
```

**Limitation:** The script is frozen at generation time. It cannot know about data that changes at runtime (like user-defined aliases stored in a config file).

### Dynamic Completions

The shell calls **your binary itself** every time the user presses `<TAB>`. Your binary inspects the partial command line and returns matching completions to stdout.

```
[runtime] user presses TAB → shell calls your binary → binary reads config → returns completions
```

This is what `clap_complete`'s `CompleteEnv` provides. It uses the `COMPLETE` environment variable as the protocol:

1. `COMPLETE=zsh web` (no args after `--`) → outputs a **registration script** (a small shell function that wires up the callback)
2. `COMPLETE=zsh web -- web gh<TAB>` → outputs **actual completion candidates** by running your `ArgValueCompleter` functions

## Dependencies

```toml
# Cargo.toml
[dependencies]
clap          = { version = "4", features = ["derive"] }
clap_complete = { version = "4", features = ["unstable-dynamic"] }
```

The `unstable-dynamic` feature flag enables `CompleteEnv` and `ArgValueCompleter`.

## Implementation Step by Step

### 1. Register the completion hook in `main()`

This **must** be the first thing in `main()`, before argument parsing. When the `COMPLETE` env var is set, the binary outputs completion data and exits immediately — it never reaches your normal logic.

```rust
// main.rs
use clap_complete::CompleteEnv;

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();
    // ... normal CLI logic below ...
}
```

### 2. Write a custom completer function

This function is called at completion time. It receives the current partial input and returns matching candidates. Since it runs every time the user presses `<TAB>`, it reads the config file live — completions are always up to date.

```rust
// config.rs
use clap_complete::engine::CompletionCandidate;

pub fn complete_alias(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };
    let Ok(config) = load() else {
        return vec![];
    };
    config
        .aliases
        .into_keys()
        .filter(|alias| alias.starts_with(current))
        .map(CompletionCandidate::new)
        .collect()
}
```

Key points:
- The function signature must be `fn(&OsStr) -> Vec<CompletionCandidate>`.
- **Silently return an empty vec on errors.** Never panic or print errors — this runs in the background during tab completion and any stderr output or crash will confuse the shell.

### 3. Attach the completer to arguments

Use `#[arg(add = ArgValueCompleter::new(...))]` on arguments that need custom completions:

```rust
// cli.rs
use clap_complete::engine::ArgValueCompleter;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Alias to open
    #[arg(
        value_hint = ValueHint::Other,
        add = ArgValueCompleter::new(complete_alias)
    )]
    pub alias: Option<String>,
}
```

### 4. Provide a `completions` subcommand for easy setup

Rather than requiring users to remember `source <(COMPLETE=zsh web)`, provide a `completions` subcommand that outputs the same registration script:

```rust
Some(Commands::Completions { shell }) => {
    let shell_name = match shell {
        clap_complete::Shell::Bash => "bash",
        clap_complete::Shell::Zsh => "zsh",
        clap_complete::Shell::Fish => "fish",
        clap_complete::Shell::Elvish => "elvish",
        clap_complete::Shell::PowerShell => "powershell",
        _ => anyhow::bail!("Unsupported shell: {shell}"),
    };
    std::env::set_var("COMPLETE", shell_name);
    CompleteEnv::with_factory(Cli::command)
        .try_complete(["web"], None::<&std::path::Path>)?;
}
```

This internally triggers `CompleteEnv`'s registration path — the same one used by `COMPLETE=zsh web`. Both methods produce identical output:

```sh
# These are now equivalent:
source <(COMPLETE=zsh web)
source <(web completions zsh)
web completions zsh > ~/.zfunc/_web
```

## Troubleshooting

### Problem: File/directory names appear in completions

**Cause:** Without `value_hint`, clap's completion generators fall back to the shell's default completer (`_default` in zsh), which completes file and directory names.

**Fix:** Add `value_hint = ValueHint::Other` to arguments that are not file paths:

```rust
#[arg(value_hint = ValueHint::Other)]
pub alias: Option<String>,
```

Available hints and their behavior:

| ValueHint       | Shell behavior                       |
|-----------------|--------------------------------------|
| `Unknown`       | Falls back to file/dir (the default) |
| `Other`         | No default completion                |
| `FilePath`      | Completes file paths                 |
| `DirPath`       | Completes directory paths            |
| `Url`           | URL-specific completion              |
| `Username`      | System username completion           |
| `Hostname`      | Hostname completion                  |

### Problem: Aliases don't show up (only subcommands do)

**Cause:** You're using static completions generated by `clap_complete::generate()`. Static scripts are shell code generated at build time — they have no way to call your `ArgValueCompleter` function at runtime. Your aliases exist only in the config file, so static scripts can't know about them.

**Fix:** Use dynamic completions. Replace `clap_complete::generate()` with `CompleteEnv::try_complete()` in your `completions` subcommand handler (see step 4 above). This outputs the dynamic registration script that calls your binary at completion time.

### Problem: URLs with `?` or `&` cause shell errors

**Cause:** This is a shell issue, not a Rust issue. Zsh interprets `?` as a glob character and `&` as a background operator **before** your binary receives the arguments.

**Fix:** Users must quote URLs containing special characters:

```sh
# Broken:
web add aws https://example.com/path?tenant=abc&foo=bar

# Working:
web add aws 'https://example.com/path?tenant=abc&foo=bar'
```

There is no way to fix this from the Rust side — the shell processes arguments before passing them to your program.

## How It Works Internally (Sequence Diagram)

### User setup (one-time)

```
User runs:  source <(web completions zsh)

  1. Shell executes: web completions zsh
  2. Binary sets COMPLETE=zsh, calls CompleteEnv::try_complete(["web"])
  3. CompleteEnv sees COMPLETE=zsh with no completion args
  4. Outputs a zsh function (_clap_dynamic_completer_web) + compdef registration
  5. Shell evaluates the output → completion function is now registered for "web"
```

### Every TAB press

```
User types:  web g<TAB>

  1. Zsh sees "web" → calls _clap_dynamic_completer_web
  2. The function runs:  COMPLETE=zsh web -- web g
  3. Binary's main() calls CompleteEnv::complete()
  4. CompleteEnv sees COMPLETE=zsh + args after "--"
  5. Clap parses the partial command line "web g"
  6. Finds that "g" is in the position of the alias argument
  7. Calls complete_alias("g") → reads config → returns ["gh"]
  8. Outputs "gh" to stdout
  9. Zsh displays "gh" as a completion candidate
```

### The generated zsh function (simplified)

```zsh
function _clap_dynamic_completer_web() {
    # CURRENT is zsh's cursor position (1-indexed)
    local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)

    # Call the binary with the current words on the command line
    local completions=("${(@f)$(
        COMPLETE="zsh" web -- "${words[@]}" 2>/dev/null
    )}")

    # Display the completions
    if [[ -n $completions ]]; then
        _describe 'values' completions
    fi
}

compdef _clap_dynamic_completer_web web
```

## Summary of Key Decisions

| Decision | What we chose | Why |
|----------|--------------|-----|
| Completion approach | Dynamic (`CompleteEnv`) | Aliases are runtime data from a config file |
| `completions` subcommand output | Dynamic registration script | So both setup methods give full alias completion |
| `value_hint` for alias args | `ValueHint::Other` | Prevents shell from adding file/dir noise |
| `value_hint` for URL args | `ValueHint::Url` | Semantic hint for URL-type arguments |
| Error handling in completer | Silent empty vec | Never disrupt the user's TAB experience |
