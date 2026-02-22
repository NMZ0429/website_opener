use anyhow::{Context, Result, anyhow};
use dialoguer::Select;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
}

pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(".config/web/config.toml"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file at {}", path.display()))?;
    toml::from_str(&content).with_context(|| "Failed to parse config file")
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory at {}", parent.display()))?;
    }
    let content = toml::to_string_pretty(config).with_context(|| "Failed to serialize config")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config file at {}", path.display()))?;
    Ok(())
}

pub fn parse_aliases(aliases: &str) -> Vec<&str> {
    aliases.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect()
}

pub fn add_alias(aliases: &str, url: &str) -> Result<()> {
    let mut config = load()?;
    for alias in parse_aliases(aliases) {
        config.aliases.insert(alias.to_string(), url.to_string());
    }
    save(&config)
}

pub fn remove_alias(aliases: &str) -> Result<()> {
    let mut config = load()?;
    for alias in parse_aliases(aliases) {
        if config.aliases.remove(alias).is_none() {
            anyhow::bail!("Alias '{}' not found", alias);
        }
    }
    save(&config)
}

pub fn resolve_alias(alias: &str) -> Result<String> {
    let config = load()?;
    config
        .aliases
        .get(alias)
        .cloned()
        .ok_or_else(|| anyhow!("Alias '{}' not found", alias))
}

pub fn list_aliases() -> Result<Vec<(String, String)>> {
    let config = load()?;
    Ok(config
        .aliases
        .into_iter()
        .collect())
}

pub fn import_aliases(path: &str) -> Result<()> {
    let content = if path == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .with_context(|| "Failed to read from stdin")?;
        buf
    } else {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file '{}'", path))?
    };

    let imported: Config =
        toml::from_str(&content).with_context(|| "Failed to parse TOML input")?;

    if imported.aliases.is_empty() {
        println!("No aliases found in input.");
        return Ok(());
    }

    let mut config = load()?;

    let mut new_aliases: Vec<(String, String)> = Vec::new();
    let mut conflicts: Vec<(String, String, String)> = Vec::new(); // (alias, existing_url, imported_url)
    let mut unchanged: usize = 0;

    for (alias, imported_url) in &imported.aliases {
        match config.aliases.get(alias) {
            Some(existing_url) if existing_url == imported_url => {
                unchanged += 1;
            }
            Some(existing_url) => {
                conflicts.push((alias.clone(), existing_url.clone(), imported_url.clone()));
            }
            None => {
                new_aliases.push((alias.clone(), imported_url.clone()));
            }
        }
    }

    // Apply new aliases directly
    for (alias, url) in &new_aliases {
        config.aliases.insert(alias.clone(), url.clone());
    }

    // Resolve conflicts interactively
    let mut overwritten: usize = 0;
    let mut skipped: usize = 0;
    let mut bulk_action: Option<bool> = None; // Some(true) = use all imported, Some(false) = keep all existing

    for (alias, existing_url, imported_url) in &conflicts {
        if let Some(use_imported) = bulk_action {
            if use_imported {
                config.aliases.insert(alias.clone(), imported_url.clone());
                overwritten += 1;
            } else {
                skipped += 1;
            }
            continue;
        }

        let prompt = format!(
            "Conflict for '{}':\n  current:  {}\n  imported: {}",
            alias, existing_url, imported_url
        );
        let remaining = conflicts.len() - overwritten - skipped;
        let items = if remaining > 1 {
            vec![
                format!("Keep existing ({})", existing_url),
                format!("Use imported ({})", imported_url),
                "Keep all existing".to_string(),
                "Use all imported".to_string(),
            ]
        } else {
            vec![
                format!("Keep existing ({})", existing_url),
                format!("Use imported ({})", imported_url),
            ]
        };

        let selection = Select::new()
            .with_prompt(&prompt)
            .items(&items)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                skipped += 1;
            }
            1 => {
                config.aliases.insert(alias.clone(), imported_url.clone());
                overwritten += 1;
            }
            2 => {
                // Keep all existing
                skipped += 1;
                bulk_action = Some(false);
            }
            3 => {
                // Use all imported
                config.aliases.insert(alias.clone(), imported_url.clone());
                overwritten += 1;
                bulk_action = Some(true);
            }
            _ => unreachable!(),
        }
    }

    save(&config)?;

    // Print summary
    let added = new_aliases.len();
    let mut parts: Vec<String> = Vec::new();
    if added > 0 {
        parts.push(format!("{} added", added));
    }
    if overwritten > 0 {
        parts.push(format!("{} overwritten", overwritten));
    }
    if skipped > 0 {
        parts.push(format!("{} skipped", skipped));
    }
    if unchanged > 0 {
        parts.push(format!("{} unchanged", unchanged));
    }
    if parts.is_empty() {
        println!("Nothing to import.");
    } else {
        println!("Import complete: {}.", parts.join(", "));
    }

    Ok(())
}

pub fn complete_alias(current: &std::ffi::OsStr) -> Vec<clap_complete::engine::CompletionCandidate> {
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
        .map(clap_complete::engine::CompletionCandidate::new)
        .collect()
}
