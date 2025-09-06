use crate::core::Repository;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use dirs::home_dir;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    settings: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_global() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = home_dir()
            .ok_or("Cannot find home directory")?
            .join(".aigitconfig");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_repo(repo: &Repository) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = repo.git_dir.join("config.json");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save_global(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = home_dir()
            .ok_or("Cannot find home directory")?
            .join(".aigitconfig");
        
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn save_repo(&self, repo: &Repository) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = repo.git_dir.join("config.json");
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.settings.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.settings.remove(key)
    }

    pub fn is_empty(&self) -> bool {
        self.settings.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.settings.iter()
    }

    pub fn get_user_name(&self) -> String {
        self.get("user.name")
            .cloned()
            .or_else(|| std::env::var("AIGIT_AUTHOR").ok().and_then(|s| s.split('<').next().map(|n| n.trim().to_string())))
            .unwrap_or_else(|| "AI Git User".to_string())
    }

    pub fn get_user_email(&self) -> String {
        self.get("user.email")
            .cloned()
            .or_else(|| {
                std::env::var("AIGIT_AUTHOR").ok().and_then(|s| {
                    s.split('<').nth(1).and_then(|e| e.split('>').next()).map(|e| e.trim().to_string())
                })
            })
            .unwrap_or_else(|| "ai@example.com".to_string())
    }

    pub fn get_author_string(&self) -> String {
        format!("{} <{}>", self.get_user_name(), self.get_user_email())
    }
}
