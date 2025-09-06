use clap::Subcommand;
use crate::core::{Config};
use colored::*;
use std::path::PathBuf;
use std::io::Write;

#[derive(Subcommand)]
pub enum ConfigAction {
    Set {
        key: String,
        value: String,
    },
    Get {
        key: String,
    },
    List,
    User {
        name: Option<String>,
        #[arg(long)]
        email: Option<String>,
    },
}

pub async fn run(action: &ConfigAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ConfigAction::Set { key, value } => {
            validate_config_key(key)?;
            validate_config_value(key, value)?;
            set_config(key, value).await?;
        },
        ConfigAction::Get { key } => {
            get_config(key).await?;
        },
        ConfigAction::List => {
            list_config().await?;
        },
        ConfigAction::User { name, email } => {
            set_user_config(name.as_deref(), email.as_deref()).await?;
        },
    }
    Ok(())
}

fn validate_config_key(key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let allowed_keys = [
        "user.name", "user.email", "user.signingkey",
        "core.editor", "core.autocrlf", "core.safecrlf",
        "ai.enabled", "ai.model", "ai.temperature",
        "security.requireSignature", "security.auditLog",
        "commit.gpgsign", "commit.template"
    ];

    if !allowed_keys.contains(&key) {
        return Err(format!("Invalid configuration key: {}", key).into());
    }
    
    Ok(())
}

fn validate_config_value(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        "user.email" => {
            if !value.contains('@') || !value.contains('.') {
                return Err("Invalid email format".into());
            }
        },
        "ai.temperature" => {
            if let Ok(temp) = value.parse::<f32>() {
                if temp < 0.0 || temp > 2.0 {
                    return Err("Temperature must be between 0.0 and 2.0".into());
                }
            } else {
                return Err("Temperature must be a number".into());
            }
        },
        key if key.ends_with(".enabled") || key.ends_with("gpgsign") || key.ends_with("auditLog") => {
            match value.to_lowercase().as_str() {
                "true" | "false" | "yes" | "no" | "1" | "0" => {},
                _ => return Err("Boolean values must be true/false, yes/no, or 1/0".into()),
            }
        },
        _ => {}
    }
    
    Ok(())
}

async fn set_config(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config_path = PathBuf::from(".aigit/config.json");
    let mut config = if repo_config_path.exists() {
        Config::load_from_file(&repo_config_path)?
    } else {
        Config::load_global().unwrap_or_default()
    };
    
    config.set(key, value);
    
    if repo_config_path.parent().map(|p| p.exists()).unwrap_or(false) {
        config.save_to_file(&repo_config_path)?;
        println!("{} {} = {}", "Set".green(), key.cyan(), value);
    } else {
        config.save_global()?;
        println!("{} {} = {} (global)", "Set".green(), key.cyan(), value);
    }
    
    audit_config_change("set", key, Some(value)).await?;
    Ok(())
}

async fn get_config(key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = Config::load_from_file(&PathBuf::from(".aigit/config.json")).ok();
    let global_config = Config::load_global().unwrap_or_default();
    
    let value = repo_config
        .as_ref()
        .and_then(|c| c.get(key))
        .or_else(|| global_config.get(key));
    
    match value {
        Some(val) => {
            println!("{}", val);
            audit_config_change("get", key, None).await?;
        },
        None => println!("{} {}", "No value found for".red(), key.cyan()),
    }
    
    Ok(())
}

async fn list_config() -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = Config::load_from_file(&PathBuf::from(".aigit/config.json")).ok();
    let global_config = Config::load_global().unwrap_or_default();
    
    println!("{}", "Repository configuration:".cyan().bold());
    if let Some(config) = &repo_config {
        if config.is_empty() {
            println!("  {}", "No repository configuration found".yellow());
        } else {
            for (key, value) in config.iter() {
                println!("  {} = {}", key.cyan(), value);
            }
        }
    } else {
        println!("  {}", "No repository found".yellow());
    }
    
    println!("\n{}", "Global configuration:".cyan().bold());
    if global_config.is_empty() {
        println!("  {}", "No global configuration found".yellow());
    } else {
        for (key, value) in global_config.iter() {
            println!("  {} = {}", key.cyan(), value);
        }
    }
    
    audit_config_change("list", "", None).await?;
    Ok(())
}

async fn set_user_config(name: Option<&str>, email: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_none() && email.is_none() {
        return Err("Please provide name and/or email".into());
    }
    
    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err("Name cannot be empty".into());
        }
        set_config("user.name", n).await?;
    }
    
    if let Some(e) = email {
        if e.trim().is_empty() {
            return Err("Email cannot be empty".into());
        }
        validate_config_value("user.email", e)?;
        set_config("user.email", e).await?;
    }
    
    Ok(())
}

async fn audit_config_change(action: &str, key: &str, value: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let audit_file = PathBuf::from(".aigit/logs/audit.log");
    if !audit_file.exists() {
        return Ok(());
    }
    
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = whoami::username();
    let details = match value {
        Some(v) => format!("{}={}", key, v),
        None => key.to_string(),
    };
    
    let entry = format!("{},{},{},{},config\n", timestamp, action, user, details);
    std::fs::OpenOptions::new()
        .append(true)
        .open(audit_file)?
        .write_all(entry.as_bytes())?;
    
    Ok(())
}
