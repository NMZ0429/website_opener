mod browser;
mod cli;
mod config;
mod display;

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
        Some(Commands::Import { path }) => {
            config::import_aliases(&path)?;
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
            let term = console::Term::stdout();
            let opts = if term.is_term() {
                display::ListOptions {
                    color: console::colors_enabled(),
                    decorate: true,
                    width: term.size_checked().map(|(_, cols)| cols as usize),
                }
            } else {
                display::ListOptions::plain()
            };
            print!("{}", display::render_list(&aliases, &opts));
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
                import)
                    _arguments \
                        '1:path:_files -g "*.toml"'
                    ;;
                help)
                    local -a subcmds=(
                        'add:Register new alias(es)'
                        'completions:Generate shell completions'
                        'export:Export current alias settings to stdout'
                        'help:Print this message or the help of the given subcommand(s)'
                        'import:Import aliases from a TOML file'
                        'list:List all aliases'
                        'remove:Remove alias(es)'
                    )
                    _describe -t subcommands 'subcommand' subcmds
                    ;;
            esac
            ;;
    esac
}

_web_first_arg() {
    _web_aliases
    local -a subcommands=(
        'add:Register new alias(es) — comma-separated for multiple (e.g. claude,c)'
        'completions:Generate shell completions'
        'export:Export current alias settings to stdout (TOML format)'
        'help:Print this message or the help of the given subcommand(s)'
        'import:Import aliases from a TOML file'
        'list:List all aliases'
        'remove:Remove alias(es) — comma-separated for multiple (e.g. claude,c)'
    )
    _describe -t subcommands 'subcommand' subcommands
}

_web_aliases() {
    local -a aliases urls
    aliases=("${(@f)$(web _complete-aliases 2>/dev/null)}")
    [[ -n $aliases ]] || return 1
    # Most URLs share the https:// scheme; drop it to leave the menu more room
    aliases=("${(@)aliases/:https:\/\//:}")
    urls=("${(@)${(@)aliases#([^:\\]|\\?)##:}//\\(#b)(?)/$match[1]}")

    # zsh clips URLs that don't fit the menu; end those with "…" instead. Call
    # through to any compadd wrapper already in place (zsh-autocomplete has one).
    local saved=$functions[compadd]
    functions[_web_compadd]=${saved:-'builtin compadd "$@"'}
    {
        compadd() {
            local i=${@[(I)-d]}
            if (( i )); then
                local name=${@[i+1]} j url
                local -a disp=("${(@P)name}")
                for j in {1..$#disp}; do
                    [[ $disp[j] == *'-- '* ]] || continue
                    url=${${disp[j]#*-- }%%[[:space:]]##}
                    (( $urls[(Ie)$url] )) || disp[j]="${disp[j][1,-2]}…"
                done
                set -A $name "$disp[@]"
            fi
            _web_compadd "$@"
        }
        _describe -t aliases 'alias' aliases
    } always {
        unfunction compadd _web_compadd
        [[ -n $saved ]] && functions[compadd]=$saved
    }
}

# Color aliases like `web list`, unless a style is already set for them. zsh
# matches these against each alias, each "-- url" cell and each "alias  -- url" row.
zstyle -g _web_colors ':completion:*:*:web:*:aliases' list-colors ||
    zstyle ':completion:*:*:web:*:aliases' list-colors \
        '=(#b)(-- )([a-z]##://|)([^/?#]#)(*)=2=2=2=0=2' \
        '=(#b)(*)( -- )([a-z]##://|)([^/?#]#)(*)=2=1;36=2=2=0=2' \
        '=(#b)(*)=0=1;36'
unset _web_colors

_web "$@"
"#
}

