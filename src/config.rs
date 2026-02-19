use anyhow::{Context, Result, anyhow};
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
