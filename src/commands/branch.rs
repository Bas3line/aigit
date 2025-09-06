use crate::core::{Repository, Branch, Config};
use crate::ai::gemini::GeminiClient;
use crate::utils::analyzer::analyze_codebase;
use std::fs;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;

pub async fn run(
    name: Option<String>, 
    delete: Option<String>, 
    ai_suggest: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let config = Config::load_repo(&repo).unwrap_or_else(|_| Config::load_global().unwrap_or_default());

    if let Some(branch_name) = delete {
        delete_branch(&repo, &branch_name, &config).await?;
        return Ok(());
    }

    if ai_suggest {
        suggest_branch_names(&repo).await?;
        return Ok(());
    }

    if let Some(branch_name) = name {
        validate_branch_name(&branch_name)?;
        create_branch(&repo, &branch_name, &config).await?;
    } else {
        list_branches(&repo, &config).await?;
    }
    
    Ok(())
}

fn validate_branch_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_empty() {
        return Err("Branch name cannot be empty".into());
    }
    
    if name.len() > 100 {
        return Err("Branch name too long (max 100 characters)".into());
    }
    
    let invalid_chars = ['~', '^', ':', '?', '*', '[', '\\', ' ', '\t', '\n'];
    if name.chars().any(|c| invalid_chars.contains(&c)) {
        return Err("Branch name contains invalid characters".into());
    }
    
    if name.starts_with('-') || name.ends_with('.') || name.contains("..") {
        return Err("Invalid branch name format".into());
    }
    
    let reserved_names = ["HEAD", "ORIG_HEAD", "FETCH_HEAD", "MERGE_HEAD"];
    if reserved_names.contains(&name) {
        return Err("Branch name is reserved".into());
    }
    
    Ok(())
}

async fn create_branch(repo: &Repository, name: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let branch_path = repo.heads_dir().join(name);
    
    if branch_path.exists() {
        return Err(format!("Branch '{}' already exists", name).into());
    }

    let head_commit = Branch::get_current_commit(repo);
    // let branch_id = generate_branch_id(name, &head_commit);
    
    match &head_commit {
        Some(commit_hash) => {
            fs::write(&branch_path, commit_hash)?;
            println!("{} {} {} {}", 
                    "Created branch:".green(), 
                    name.bright_cyan(),
                    "at".bright_black(),
                    commit_hash[..8].bright_yellow());
        },
        None => {
            fs::write(&branch_path, "")?;
            println!("{} {} {}", 
                    "Created empty branch:".green(), 
                    name.bright_cyan(), 
                    "(no commits yet)".bright_black());
        }
    }
    
    audit_branch_operation("create", name, &head_commit, config).await?;
    Ok(())
}

async fn delete_branch(repo: &Repository, name: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if name == "main" || name == "master" {
        println!("{}", "Warning: Attempting to delete default branch".yellow());
        println!("{}", "Are you sure? (y/N)".yellow());
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "Branch deletion aborted".yellow());
            return Ok(());
        }
    }
    
    let current_branch = Branch::get_current_branch(repo);
    
    if current_branch.as_deref() == Some(name) {
        return Err("Cannot delete the currently checked out branch".into());
    }

    let branch_path = repo.heads_dir().join(name);
    
    if !branch_path.exists() {
        return Err(format!("Branch '{}' does not exist", name).into());
    }
    
    let branch_commit = fs::read_to_string(&branch_path).ok();
    
    fs::remove_file(&branch_path)?;
    println!("{} {}", "Deleted branch:".green(), name.bright_red());
    
    audit_branch_operation("delete", name, &branch_commit, config).await?;
    Ok(())
}

async fn list_branches(repo: &Repository, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let current_branch = Branch::get_current_branch(repo);
    let branches = Branch::list(repo)?;
    
    if branches.is_empty() {
        println!("{}", "No branches found".yellow());
        return Ok(());
    }
    
    println!("{}", "Branches:".cyan().bold());
    
    for branch in &branches {
        let is_current = current_branch.as_deref() == Some(&branch.name);
        let prefix = if is_current { "* " } else { "  " };
        
        let branch_display = if is_current {
            branch.name.green().bold()
        } else {
            branch.name.white()
        };
        
        match &branch.hash {
            Some(hash) if !hash.is_empty() => {
                let commit_info = get_commit_summary(repo, hash).unwrap_or_else(|| "invalid commit".to_string());
                println!("{}{} {} {}", 
                        prefix, 
                        branch_display, 
                        hash[..8].bright_yellow(), 
                        commit_info.bright_black());
            },
            _ => {
                println!("{}{} {}", prefix, branch_display, "(no commits)".bright_black());
            }
        }
    }
    
    if config.get("security.auditLog").map(|v| v == "true").unwrap_or(false) {
        println!("\n{} Branch operations are being audited", "🔍".cyan());
    }
    
    Ok(())
}

async fn suggest_branch_names(repo: &Repository) -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("AI analyzing project for branch suggestions...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let context = analyze_codebase(repo).await;
    let gemini = GeminiClient::new();

    match gemini.suggest_branch_name(&context).await {
        Ok(suggestions) => {
            pb.finish_and_clear();
            println!("{}", "AI suggested branch names:".cyan().bold());
            for (i, suggestion) in suggestions.iter().enumerate() {
                let category = categorize_branch_name(suggestion);
                println!("{}. {} {}", 
                        i + 1, 
                        suggestion.bright_green(), 
                        format!("({})", category).bright_black());
            }
            
            println!("\n{}", "Branch naming conventions:".cyan());
            println!("  {} New features", "feature/name".bright_blue());
            println!("  {} Bug fixes", "bugfix/issue".bright_red());
            println!("  {} Critical fixes", "hotfix/critical".bright_magenta());
            println!("  {} Code improvements", "refactor/component".bright_yellow());
            println!("  {} Experiments", "experimental/idea".bright_purple());
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Failed to generate suggestions: {}", e).into());
        }
    }
    
    Ok(())
}

fn categorize_branch_name(name: &str) -> &str {
    if name.starts_with("feature/") { "feature" }
    else if name.starts_with("bugfix/") || name.starts_with("fix/") { "bugfix" }
    else if name.starts_with("hotfix/") { "hotfix" }
    else if name.starts_with("refactor/") { "refactor" }
    else if name.starts_with("experimental/") || name.starts_with("exp/") { "experimental" }
    else if name.starts_with("release/") { "release" }
    else { "general" }
}

// fn generate_branch_id(name: &str, commit_hash: &Option<String>) -> String {
//     let content = format!("{}{}{}", name, commit_hash.as_deref().unwrap_or(""), chrono::Utc::now().to_rfc3339());
//     hex::encode(digest::digest(&digest::SHA256, content.as_bytes()).as_ref())[..8].to_string()
// }

fn get_commit_summary(repo: &Repository, hash: &str) -> Option<String> {
    use crate::core::Object;
    let content = Object::read(repo, hash).ok()?;
    let commit: crate::core::Commit = serde_json::from_slice(&content).ok()?;
    Some(commit.short_message())
}

async fn audit_branch_operation(
    operation: &str, 
    branch_name: &str, 
    commit_hash: &Option<String>, 
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.get("security.auditLog").map(|v| v == "true").unwrap_or(false) {
        return Ok(());
    }
    
    let audit_file = std::path::PathBuf::from(".aigit/logs/audit.log");
    if !audit_file.exists() {
        return Ok(());
    }
    
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = whoami::username();
    let details = match commit_hash {
        Some(hash) => format!("{}:{}", branch_name, hash),
        None => branch_name.to_string(),
    };
    
    let entry = format!("{},{},{},{},branch\n", timestamp, operation, user, details);
    std::fs::OpenOptions::new()
        .append(true)
        .open(audit_file)?
        .write_all(entry.as_bytes())?;
    
    Ok(())
}
