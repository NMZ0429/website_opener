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
            clap_complete::generate(shell, &mut Cli::command(), "web", &mut std::io::stdout());
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
