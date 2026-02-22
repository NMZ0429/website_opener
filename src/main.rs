mod browser;
mod cli;
mod config;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use cli::{Cli, Commands};

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Add { aliases, url }) => {
            let names = config::parse_aliases(&aliases);
            config::add_alias(&aliases, &url)?;
            let quoted: Vec<String> = names.iter().map(|a| format!("'{a}'")).collect();
            println!("Added {} -> {url}", quoted.join(", "));
        }
        Some(Commands::Remove { aliases }) => {
            let names = config::parse_aliases(&aliases);
            config::remove_alias(&aliases)?;
            let quoted: Vec<String> = names.iter().map(|a| format!("'{a}'")).collect();
            println!("Removed {}", quoted.join(", "));
        }
        Some(Commands::Completions { shell }) => {
            if shell == clap_complete::Shell::Zsh {
                print!("{}", zsh_completion_script());
            } else {
                let shell_name = match shell {
                    clap_complete::Shell::Bash => "bash",
                    clap_complete::Shell::Fish => "fish",
                    clap_complete::Shell::Elvish => "elvish",
                    clap_complete::Shell::PowerShell => "powershell",
                    _ => anyhow::bail!("Unsupported shell: {shell}"),
                };
                std::env::set_var("COMPLETE", shell_name);
                CompleteEnv::with_factory(Cli::command)
                    .try_complete(["web"], None::<&std::path::Path>)?;
            }
        }
        Some(Commands::Export) => {
            let config = config::load()?;
            print!("{}", toml::to_string_pretty(&config)?);
        }
        Some(Commands::CompleteAliases) => {
            let aliases = config::list_aliases()?;
            for (alias, url) in aliases {
                // Escape colons and backslashes for zsh _describe format
                let alias = alias.replace('\\', "\\\\").replace(':', "\\:");
                let url = url.replace('\\', "\\\\");
                println!("{alias}:{url}");
            }
        }
        Some(Commands::List) => {
            let aliases = config::list_aliases()?;
            if aliases.is_empty() {
                println!("No aliases registered.");
            } else {
                // Group aliases by URL
                let mut by_url: std::collections::BTreeMap<String, Vec<String>> =
                    std::collections::BTreeMap::new();
                for (alias, url) in aliases {
                    by_url.entry(url).or_default().push(alias);
                }
                let rows: Vec<(String, String)> = by_url
                    .into_iter()
                    .map(|(url, names)| (names.join(", "), url))
                    .collect();
                let max_len = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
                for (names, url) in rows {
                    println!("{:<width$}  {}", names, url, width = max_len);
                }
            }
        }
        None => {
            let alias = cli
                .alias
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("No alias provided. Use `web --help` for usage."))?;
            let url = config::resolve_alias(alias)?;
            browser::open_url(&url, cli.browser_choice())?;
        }
    }
    Ok(())
}

fn zsh_completion_script() -> &'static str {
    r#"#compdef web

_web() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    _arguments -s -S \
        '(--chrome --firefox --brave)--safari[Use Safari browser]' \
        '(--safari --firefox --brave)--chrome[Use Chrome browser]' \
        '(--safari --chrome --brave)--firefox[Use Firefox browser]' \
        '(--safari --chrome --firefox)--brave[Use Brave browser]' \
        '(- *)--help[Print help]' \
        '(- *)--version[Print version]' \
        '1: :_web_first_arg' \
        '*:: :->subcmd' \
        && return

    case $state in
        subcmd)
            case $line[1] in
                add)
                    _arguments \
                        '1:aliases:' \
                        '2:url:_urls'
                    ;;
                remove)
                    _arguments \
                        '1:aliases:_web_aliases'
                    ;;
                completions)
                    _arguments \
                        '1:shell:(bash zsh fish elvish powershell)'
                    ;;
                help)
                    local -a subcmds=(
                        'add:Register new alias(es)'
                        'completions:Generate shell completions'
                        'help:Print this message or the help of the given subcommand(s)'
                        'list:List all aliases'
                        'remove:Remove alias(es)'
                    )
                    _describe 'subcommand' subcmds
                    ;;
            esac
            ;;
    esac
}

_web_first_arg() {
    local -a subcommands=(
        'add:Register new alias(es) — comma-separated for multiple (e.g. claude,c)'
        'completions:Generate shell completions'
        'help:Print this message or the help of the given subcommand(s)'
        'list:List all aliases'
        'remove:Remove alias(es) — comma-separated for multiple (e.g. claude,c)'
    )
    _describe 'subcommand' subcommands
    _web_aliases
}

_web_aliases() {
    local -a aliases
    aliases=("${(@f)$(web _complete-aliases 2>/dev/null)}")
    [[ -n $aliases ]] && _describe 'alias' aliases
}

_web "$@"
"#
}

