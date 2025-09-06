use crate::core::{Repository, Index, Object, ObjectType, Commit, Tree, Config};
use crate::ai::gemini::GeminiClient;
use crate::utils::diff::get_staged_diff;
use chrono::Utc;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use ring::digest;
use std::io::Write;

pub async fn run(
    message: Option<String>, 
    amend: bool, 
    ai_review: bool, 
    signoff: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let mut index = Index::load(&repo)?;
    let config = Config::load_repo(&repo).unwrap_or_else(|_| Config::load_global().unwrap_or_default());

    if index.entries.is_empty() && !amend {
        println!("{}", "Nothing to commit".yellow());
        return Ok(());
    }

    if index.has_conflicts() {
        println!("{}", "Cannot commit with unresolved conflicts".red());
        let conflicts = index.get_conflicted_files();
        for file in conflicts {
            println!("  {}", file.red());
        }
        return Err("Unresolved conflicts".into());
    }

    security_pre_commit_checks(&index).await?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());

    if ai_review {
        pb.set_message("AI reviewing changes...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        
        let diff_content = get_staged_diff(&repo, &index).await;
        let gemini = GeminiClient::new();
        
        match gemini.review_code(&diff_content).await {
            Ok(review) => {
                pb.finish_and_clear();
                println!("\n{}", "AI Code Review:".cyan().bold());
                println!("{}", review);
                println!("\n{}", "Proceed with commit? (y/N)".yellow());
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("{}", "Commit aborted".red());
                    return Ok(());
                }
            },
            Err(e) => {
                pb.finish_and_clear();
                println!("{} {}", "AI review failed:".yellow(), e);
            }
        }
    }

    let commit_message = match message {
        Some(msg) => {
            validate_commit_message(&msg)?;
            msg
        },
        None => {
            pb.set_message("Generating AI commit message...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            
            let diff_content = get_staged_diff(&repo, &index).await;
            let gemini = GeminiClient::new();
            
            match gemini.generate_commit_message(&diff_content).await {
                Ok(ai_msg) => {
                    pb.finish_and_clear();
                    println!("{} {}", "AI suggested:".cyan(), ai_msg.bright_white());
                    println!("{}", "Use this message? (Y/n/e)dit".yellow());
                    
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    
                    match input.trim().to_lowercase().as_str() {
                        "n" | "no" => {
                            println!("{}", "Enter commit message:".yellow());
                            let mut manual_msg = String::new();
                            std::io::stdin().read_line(&mut manual_msg)?;
                            let msg = manual_msg.trim().to_string();
                            validate_commit_message(&msg)?;
                            msg
                        },
                        "e" | "edit" => {
                            edit_commit_message(&ai_msg, &config)?
                        },
                        _ => {
                            validate_commit_message(&ai_msg)?;
                            ai_msg
                        }
                    }
                },
                Err(_) => {
                    pb.finish_and_clear();
                    println!("{}", "Enter commit message:".yellow());
                    let mut manual_msg = String::new();
                    std::io::stdin().read_line(&mut manual_msg)?;
                    let msg = manual_msg.trim().to_string();
                    validate_commit_message(&msg)?;
                    msg
                }
            }
        }
    };

    let final_message = if signoff {
        add_signoff(commit_message, &config)
    } else {
        commit_message
    };

    pb.set_message("Creating commit...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let tree_hash = Tree::create_from_index(&repo, &index)?;
    let parent = if amend {
        get_previous_commit_parent(&repo)
    } else {
        get_last_commit(&repo)
    };

    let author_name = config.get_user_name();
    let author_email = config.get_user_email();
    // let timestamp = Utc::now();

    let commit = Commit::new_secure(
        tree_hash.clone(),
        parent,
        author_name.clone(),
        author_email,
        final_message.clone(),
        generate_commit_signature(&final_message, &tree_hash)?,
    );

    let commit_content = serde_json::to_string(&commit)?;
    let commit_hash = Object::create(&repo, ObjectType::Commit, commit_content.as_bytes())?;
    
    update_head(&repo, &commit_hash);
    index.clear(&repo)?;
    
    pb.finish_and_clear();
    println!("{} {}", "Committed:".green().bold(), commit_hash[..8].bright_yellow());
    println!("{} {}", "Message:".cyan(), final_message.lines().next().unwrap_or("").bright_white());
    
    audit_commit(&commit_hash, &final_message, &author_name).await?;
    
    Ok(())
}

fn validate_commit_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".into());
    }
    
    if message.len() > 10000 {
        return Err("Commit message too long (max 10000 characters)".into());
    }
    
    let first_line = message.lines().next().unwrap_or("");
    if first_line.len() > 80 {
        println!("{}", "Warning: First line is longer than 80 characters".yellow());
    }
    
    let suspicious_patterns = [
        r"(?i)(password|secret|key|token)\s*[:=]\s*\S+",
        r"(?i)fuck|shit|damn|crap",
        r"[a-zA-Z0-9+/]{40,}={0,2}",
    ];
    
    for pattern in &suspicious_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(message) {
                println!("{}", "Warning: Commit message may contain sensitive or inappropriate content".yellow());
                break;
            }
        }
    }
    
    Ok(())
}

fn edit_commit_message(initial_message: &str, config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let editor = config.get("core.editor")
        .cloned()
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "nano".to_string());
    
    let temp_file = format!(".aigit/COMMIT_EDITMSG_{}", uuid::Uuid::new_v4());
    std::fs::write(&temp_file, initial_message)?;
    
    let status = std::process::Command::new(editor)
        .arg(&temp_file)
        .status()?;
    
    if !status.success() {
        std::fs::remove_file(&temp_file)?;
        return Err("Editor exited with error".into());
    }
    
    let edited_message = std::fs::read_to_string(&temp_file)?;
    std::fs::remove_file(&temp_file)?;
    
    let cleaned_message = edited_message
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    
    validate_commit_message(&cleaned_message)?;
    Ok(cleaned_message)
}

fn add_signoff(message: String, config: &Config) -> String {
    let signoff = format!("Signed-off-by: {} <{}>", 
                         config.get_user_name(), 
                         config.get_user_email());
    
    if message.contains(&signoff) {
        message
    } else {
        format!("{}\n\n{}", message, signoff)
    }
}

fn generate_commit_signature(message: &str, tree_hash: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = format!("{}\n{}\n{}", message, tree_hash, Utc::now().to_rfc3339());
    let signature = hex::encode(digest::digest(&digest::SHA256, content.as_bytes()).as_ref());
    Ok(signature)
}

async fn security_pre_commit_checks(index: &Index) -> Result<(), Box<dyn std::error::Error>> {
    let staged_files = index.entries.len();
    let total_size: u64 = index.metadata.values().map(|m| m.size).sum();
    
    if staged_files > 10000 {
        return Err("Too many files in single commit (max 10000)".into());
    }
    
    if total_size > 1_073_741_824 {
        return Err("Commit size too large (max 1GB)".into());
    }
    
    for (file_path, entry) in &index.metadata {
        if std::path::Path::new(file_path).exists() {
            let current_content = std::fs::read(file_path)?;
            let current_checksum = hex::encode(digest::digest(&digest::SHA256, &current_content).as_ref());
            
            if entry.checksum != current_checksum {
                return Err(format!("File {} was modified after staging", file_path).into());
            }
        }
    }
    
    Ok(())
}

fn get_last_commit(repo: &Repository) -> Option<String> {
    std::fs::read_to_string(format!("{}/.aigit/HEAD", repo.path.display()))
        .ok()
        .and_then(|content| {
            if content.starts_with("ref: ") {
                let ref_path = content.trim().strip_prefix("ref: ")?;
                std::fs::read_to_string(format!("{}/.aigit/{}", repo.path.display(), ref_path)).ok()
            } else {
                Some(content)
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn get_previous_commit_parent(repo: &Repository) -> Option<String> {
    get_last_commit(repo).and_then(|hash| {
        let commit_content = Object::read(repo, &hash).ok()?;
        let commit: Commit = serde_json::from_slice(&commit_content).ok()?;
        commit.parent
    })
}

fn update_head(repo: &Repository, commit_hash: &str) {
    let head_content = std::fs::read_to_string(format!("{}/.aigit/HEAD", repo.path.display())).unwrap();
    if head_content.starts_with("ref: ") {
        let ref_path = head_content.trim().strip_prefix("ref: ").unwrap();
        std::fs::write(format!("{}/.aigit/{}", repo.path.display(), ref_path), commit_hash).unwrap();
    }
}

async fn audit_commit(commit_hash: &str, message: &str, author: &str) -> Result<(), Box<dyn std::error::Error>> {
    let audit_file = std::path::PathBuf::from(".aigit/logs/audit.log");
    if !audit_file.exists() {
        return Ok(());
    }
    
    let timestamp = chrono::Utc::now().to_rfc3339();
    let message_summary = message.lines().next().unwrap_or("").chars().take(50).collect::<String>();
    let details = format!("{}:{}", commit_hash, message_summary);
    
    let entry = format!("{},commit,{},{},commit\n", timestamp, author, details);
    std::fs::OpenOptions::new()
        .append(true)
        .open(audit_file)?
        .write_all(entry.as_bytes())?;
    
    Ok(())
}
