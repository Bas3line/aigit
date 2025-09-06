use crate::core::{Repository, Index, Config};
use crate::utils::ignore::GitIgnore;
use std::collections::{HashMap};
use walkdir::WalkDir;
use colored::*;
use ring::digest;

pub async fn run(porcelain: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let index = Index::load(&repo)?;
    let ignore = GitIgnore::new(&repo.path);
    let config = Config::load_repo(&repo).unwrap_or_else(|_| Config::load_global().unwrap_or_default());
    
    let mut staged: HashMap<String, String> = index.entries.clone();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();
    let mut untracked = Vec::new();
    let mut conflicted = Vec::new();
    let mut corrupted = Vec::new();

    for file in index.get_conflicted_files() {
        conflicted.push(file);
    }

    for entry in WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.path().starts_with(".aigit"))
    {
        let path = entry.path();
        if ignore.is_ignored(path) {
            continue;
        }
        
        let path_str = path.to_str().unwrap();
        
        if let Some(staged_hash) = index.entries.get(path_str) {
            match std::fs::read(path) {
                Ok(current_content) => {
                    let current_hash = crate::core::object::hash_content(&current_content);
                    
                    if &current_hash != staged_hash {
                        if let Some(metadata) = index.metadata.get(path_str) {
                            let current_checksum = hex::encode(digest::digest(&digest::SHA256, &current_content).as_ref());
                            if metadata.checksum != current_checksum {
                                modified.push(path_str.to_string());
                            } else {
                                corrupted.push(path_str.to_string());
                            }
                        } else {
                            modified.push(path_str.to_string());
                        }
                    }
                },
                Err(_) => deleted.push(path_str.to_string()),
            }
            staged.remove(path_str);
        } else {
            untracked.push(path_str.to_string());
        }
    }

    for file in staged.keys() {
        if !std::path::Path::new(file).exists() {
            deleted.push(file.clone());
        }
    }

    if porcelain {
        print_porcelain_status(&staged, &modified, &deleted, &untracked, &conflicted, &corrupted);
    } else {
        print_human_status(&staged, &modified, &deleted, &untracked, &conflicted, &corrupted, &repo, &config).await;
    }
    
    Ok(())
}

fn print_porcelain_status(
    staged: &HashMap<String, String>,
    modified: &[String],
    deleted: &[String],
    untracked: &[String],
    conflicted: &[String],
    corrupted: &[String],
) {
    for file in conflicted {
        println!("UU {}", file);
    }
    
    for file in corrupted {
        println!("XX {}", file);
    }
    
    for file in staged.keys() {
        if deleted.contains(file) {
            println!("D  {}", file);
        } else {
            println!("A  {}", file);
        }
    }

    for file in modified {
        println!(" M {}", file);
    }

    for file in deleted {
        if !staged.contains_key(file) {
            println!(" D {}", file);
        }
    }

    for file in untracked {
        println!("?? {}", file);
    }
}

async fn print_human_status(
    staged: &HashMap<String, String>,
    modified: &[String],
    deleted: &[String],
    untracked: &[String],
    conflicted: &[String],
    corrupted: &[String],
    repo: &Repository,
    config: &Config,
) {
    let current_branch = get_current_branch(repo);
    let commit_count = get_commit_count(repo);
    let repo_id = get_repo_id(repo);
    
    println!("On branch {} {}", current_branch.bright_cyan(), format!("({})", repo_id).bright_black());
    println!("Total commits: {}", commit_count.to_string().bright_yellow());
    
    if config.get("security.auditLog").map(|v| v == "true").unwrap_or(false) {
        println!("{} Security audit logging enabled", "🔒".green());
    }

    if !corrupted.is_empty() {
        println!("\n{}", "Files with integrity issues:".red().bold());
        for file in corrupted {
            println!("  {} {}", "corrupted:".red(), file);
        }
        println!("{}", "Run 'aigit fsck' to verify repository integrity".red());
    }

    if !conflicted.is_empty() {
        println!("\n{}", "You have unmerged paths.".red().bold());
        println!("{}", "Unmerged paths:".red());
        for file in conflicted {
            println!("  {} {}", "both modified:".red(), file);
        }
        println!("{}", "Use 'aigit add/rm <file>...' to mark resolution".red());
    }

    if !staged.is_empty() {
        println!("\n{}", "Changes to be committed:".green());
        for file in staged.keys() {
            if deleted.contains(file) {
                println!("  {} {}", "deleted:".red(), file);
            } else {
                println!("  {} {}", "new file:".green(), file);
            }
        }
    }

    if !modified.is_empty() || (!deleted.is_empty() && staged.is_empty()) {
        println!("\n{}", "Changes not staged for commit:".yellow());
        for file in modified {
            println!("  {} {}", "modified:".yellow(), file);
        }
        for file in deleted {
            if !staged.contains_key(file) {
                println!("  {} {}", "deleted:".red(), file);
            }
        }
        println!("{}", "Use 'aigit add <file>...' to update what will be committed".yellow());
    }

    if !untracked.is_empty() {
        println!("\n{}", "Untracked files:".bright_black());
        let mut shown = 0;
        for file in untracked {
            if shown < 20 {
                println!("  {}", file.bright_black());
                shown += 1;
            } else {
                println!("  {} ({} more files)", "...".bright_black(), untracked.len() - shown);
                break;
            }
        }
        println!("\n{}", "Use 'aigit add <file>...' to include in what will be committed".bright_black());
    }

    if staged.is_empty() && modified.is_empty() && deleted.is_empty() && untracked.is_empty() && conflicted.is_empty() {
        println!("\n{}", "Working tree clean".green());
        
        if let Some(last_commit) = get_last_commit_info(repo) {
            println!("Last commit: {}", last_commit.bright_black());
        }
    }

    print_security_status(repo).await;
}

fn get_current_branch(repo: &Repository) -> String {
    std::fs::read_to_string(format!("{}/.aigit/HEAD", repo.path.display()))
        .ok()
        .and_then(|content| {
            content.strip_prefix("ref: refs/heads/")
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "detached HEAD".to_string())
}

fn get_commit_count(repo: &Repository) -> usize {
    let objects_dir = repo.objects_dir();
    if !objects_dir.exists() {
        return 0;
    }
    
    walkdir::WalkDir::new(&objects_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

fn get_repo_id(repo: &Repository) -> String {
    std::fs::read_to_string(repo.git_dir.join("info/repo-id"))
        .unwrap_or_else(|_| "unknown".to_string())
        .chars()
        .take(8)
        .collect()
}

fn get_last_commit_info(repo: &Repository) -> Option<String> {
    let last_hash = std::fs::read_to_string(format!("{}/.aigit/HEAD", repo.path.display()))
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
        .filter(|s| !s.is_empty())?;
    
    Some(format!("{} ({})", &last_hash[..8], "timestamp unavailable"))
}

async fn print_security_status(repo: &Repository) {
    let security_file = repo.git_dir.join("security/config.json");
    if security_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&security_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if config.get("audit_log").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("\n{} Audit logging active", "🔍".cyan());
                }
                if config.get("encrypt_objects").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("{} Object encryption enabled", "🔐".green());
                }
            }
        }
    }
}
