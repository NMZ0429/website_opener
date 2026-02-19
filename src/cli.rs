use clap::{Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;

use crate::config::complete_alias;

#[derive(Debug, Parser)]
#[command(name = "web", version, about = "Open URL aliases in a browser")]
pub struct Cli {
    #[arg(long, group = "browser_choice")]
    pub safari: bool,
    #[arg(long, group = "browser_choice")]
    pub chrome: bool,
    #[arg(long, group = "browser_choice")]
    pub firefox: bool,
    #[arg(long, group = "browser_choice")]
    pub brave: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Alias to open (when no subcommand given)
    #[arg(add = ArgValueCompleter::new(complete_alias))]
    pub alias: Option<String>,
}

impl Cli {
    pub fn browser_choice(&self) -> BrowserChoice {
        if self.safari {
            BrowserChoice::Safari
        } else if self.chrome {
            BrowserChoice::Chrome
        } else if self.firefox {
            BrowserChoice::Firefox
        } else if self.brave {
            BrowserChoice::Brave
        } else {
            BrowserChoice::Default
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Register new alias(es) — comma-separated for multiple (e.g. claude,c)
    Add { aliases: String, url: String },
    /// Remove alias(es) — comma-separated for multiple (e.g. claude,c)
    Remove {
        #[arg(add = ArgValueCompleter::new(complete_alias))]
        aliases: String,
    },
    /// List all aliases
    List,
    /// Generate static shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserChoice {
    Default,
    Safari,
    Chrome,
    Firefox,
    Brave,
}
