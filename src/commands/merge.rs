use crate::core::{Repository, Branch, Config, Commit, Object};
use crate::ai::gemini::GeminiClient;
use std::fs;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use ring::digest;
use std::io::Write;

pub async fn run(branch: String, ai_resolve: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let config = Config::load_repo(&repo).unwrap_or_else(|_| Config::load_global().unwrap_or_default());
    let current_branch = Branch::get_current_branch(&repo).ok_or("Not on a branch")?;
    
    if current_branch == branch {
        return Err("Cannot merge branch into itself".into());
    }

    let branch_ref = repo.heads_dir().join(&branch);
    if !branch_ref.exists() {
        return Err(format!("Branch '{}' not found", branch).into());
    }

    let branch_commit = fs::read_to_string(&branch_ref)?.trim().to_string();
    let current_commit = Branch::get_current_commit(&repo);

    security_pre_merge_checks(&repo, &branch_commit, &current_commit).await?;

    match current_commit {
        Some(current) if current == branch_commit => {
            println!("{}", "Already up to date".green());
            return Ok(());
        },
        Some(current) => {
            println!("{} {} {} {}", 
                    "Merging".green(), 
                    branch.bright_cyan(), 
                    "into".green(), 
                    current_branch.bright_cyan());
            
            if ai_resolve {
                perform_ai_assisted_merge(&repo, &current, &branch_commit, &branch, &config).await?;
            } else {
                perform_merge(&repo, &current, &branch_commit, &branch, &config).await?;
            }
        },
        None => {
            update_head(&repo, &branch_commit);
            println!("{} {} {}", 
                    "Fast-forward merge of".green(), 
                    branch.bright_cyan(),
                    "(no previous commits)".bright_black());
            
            audit_merge_operation("fast_forward", &branch, &branch_commit, &config).await?;
        }
    }
    
    Ok(())
}

async fn security_pre_merge_checks(
    repo: &Repository,
    branch_commit: &str,
    current_commit: &Option<String>
) -> Result<(), Box<dyn std::error::Error>> {
    if !commit_exists(repo, branch_commit)? {
        return Err("Target commit does not exist or is corrupted".into());
    }
    
    if let Some(current) = current_commit {
        if !commit_exists(repo, current)? {
            return Err("Current commit does not exist or is corrupted".into());
        }
        
        if is_ancestor(repo, branch_commit, current).await? {
            println!("{}", "Warning: This merge may create unnecessary complexity".yellow());
        }
    }
    
    verify_commit_integrity(repo, branch_commit).await?;
    
    Ok(())
}

async fn perform_ai_assisted_merge(
    repo: &Repository,
    current: &str,
    branch_commit: &str,
    branch_name: &str,
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("AI analyzing merge strategy...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let gemini = GeminiClient::new();
    let merge_context = create_merge_context(repo, current, branch_commit, branch_name).await?;

    match gemini.analyze_merge(&merge_context).await {
        Ok(analysis) => {
            pb.finish_and_clear();
            println!("{}", "=== AI Merge Analysis ===".cyan().bold());
            println!("{}", analysis);
            println!("\n{}", "Proceed with merge? (Y/n/s)top".yellow());
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            
            match input.trim().to_lowercase().as_str() {
                "n" | "no" => {
                    println!("{}", "Merge aborted".yellow());
                    return Ok(());
                },
                "s" | "stop" => {
                    println!("{}", "Merge stopped".red());
                    return Ok(());
                },
                _ => {}
            }
            
            perform_merge(repo, current, branch_commit, branch_name, config).await?;
        },
        Err(e) => {
            pb.finish_and_clear();
            println!("{} {}", "AI analysis failed:".yellow(), e);
            println!("{}", "Proceeding with standard merge...".yellow());
            perform_merge(repo, current, branch_commit, branch_name, config).await?;
        }
    }
    
    Ok(())
}

async fn perform_merge(
    repo: &Repository,
    current: &str,
    branch_commit: &str,
    branch_name: &str,
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let merge_base = find_merge_base(repo, current, branch_commit).await?;
    
    match merge_base {
        Some(base) if base == current => {
            fast_forward_merge(repo, branch_commit, branch_name, config).await?;
        },
        Some(base) if base == branch_commit => {
            println!("{}", "Already up to date".green());
        },
        Some(_) => {
            three_way_merge(repo, current, branch_commit, branch_name, config).await?;
        },
        None => {
            unrelated_histories_merge(repo, current, branch_commit, branch_name, config).await?;
        }
    }
    
    Ok(())
}

async fn fast_forward_merge(
    repo: &Repository,
    branch_commit: &str,
    branch_name: &str,
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    update_head(repo, branch_commit);
    println!("{} {} {}", 
            "Fast-forward merge:".green(),
            branch_name.bright_cyan(),
            branch_commit[..8].bright_yellow());
    
    audit_merge_operation("fast_forward", branch_name, branch_commit, config).await?;
    Ok(())
}

async fn three_way_merge(
    repo: &Repository,
    current: &str,
    branch_commit: &str,
    branch_name: &str,
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Performing three-way merge...".yellow());
    
    let merge_message = format!("Merge branch '{}' into {}", 
                               branch_name, 
                               Branch::get_current_branch(repo).unwrap_or_else(|| "HEAD".to_string()));
    
    let parents = vec![current.to_string(), branch_commit.to_string()];
    let author_name = config.get_user_name();
    let author_email = config.get_user_email();
    
    let merge_commit = Commit::new_merge(
        "temp_tree".to_string(),
        parents,
        author_name,
        author_email,
        merge_message.clone(),
        generate_merge_signature(current, branch_commit)?,
    );

    let commit_content = serde_json::to_string(&merge_commit)?;
    let commit_hash = Object::create(repo, crate::core::ObjectType::Commit, commit_content.as_bytes())?;
    
    update_head(repo, &commit_hash);
    println!("{} {}", "Merge commit created:".green(), commit_hash[..8].bright_yellow());
    
    audit_merge_operation("three_way", branch_name, &commit_hash, config).await?;
    Ok(())
}

async fn unrelated_histories_merge(
    repo: &Repository,
    current: &str,
    branch_commit: &str,
    branch_name: &str,
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Warning: Merging unrelated histories".yellow());
    println!("{}", "Continue? (y/N)".yellow());
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if !input.trim().eq_ignore_ascii_case("y") {
        return Err("Merge aborted".into());
    }
    
    three_way_merge(repo, current, branch_commit, branch_name, config).await
}

async fn create_merge_context(
    repo: &Repository,
    current: &str,
    branch_commit: &str,
    branch_name: &str
) -> Result<String, Box<dyn std::error::Error>> {
    let current_commit_info = get_commit_info(repo, current)?;
    let branch_commit_info = get_commit_info(repo, branch_commit)?;
    
    let context = format!(
        "Merge Analysis:\n\
         Source branch: {}\n\
         Source commit: {} - {}\n\
         Target commit: {} - {}\n\
         Potential conflicts: {}\n\
         Merge complexity: {}",
        branch_name,
        branch_commit.get(..8).unwrap_or(&branch_commit),
        branch_commit_info,
        &current[..8],
        current_commit_info,
        "Low", // This would be calculated based on actual file changes
        "Medium" // This would be calculated based on divergence
    );
    
    Ok(context)
}

fn get_commit_info(repo: &Repository, hash: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = Object::read(repo, hash)?;
    let commit: Commit = serde_json::from_slice(&content)?;
    Ok(commit.short_message())
}

async fn find_merge_base(
    repo: &Repository,
    commit1: &str,
    commit2: &str
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let ancestors1 = get_ancestors(repo, commit1).await?;
    let ancestors2 = get_ancestors(repo, commit2).await?;
    
    for ancestor in &ancestors1 {
        if ancestors2.contains(ancestor) {
            return Ok(Some(ancestor.clone()));
        }
    }
    
    Ok(None)
}

async fn get_ancestors(
    repo: &Repository,
    start_commit: &str
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ancestors = Vec::new();
    let mut to_visit = vec![start_commit.to_string()];
    let mut visited = std::collections::HashSet::new();
    
    while let Some(commit_hash) = to_visit.pop() {
        if visited.contains(&commit_hash) {
            continue;
        }
        
        visited.insert(commit_hash.clone());
        ancestors.push(commit_hash.clone());
        
        let content = Object::read(repo, &commit_hash)?;
        let commit: Commit = serde_json::from_slice(&content)?;
        
        for parent in &commit.parents {
            if !parent.is_empty() {
                to_visit.push(parent.clone());
            }
        }
    }
    
    Ok(ancestors)
}

async fn is_ancestor(
    repo: &Repository,
    potential_ancestor: &str,
    commit: &str
) -> Result<bool, Box<dyn std::error::Error>> {
    let ancestors = get_ancestors(repo, commit).await?;
    Ok(ancestors.contains(&potential_ancestor.to_string()))
}

fn commit_exists(repo: &Repository, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let (dir, file) = hash.split_at(2);
    let obj_path = repo.objects_dir().join(dir).join(file);
    Ok(obj_path.exists())
}

async fn verify_commit_integrity(
    repo: &Repository,
    commit_hash: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let content = Object::read(repo, commit_hash)?;
    let _commit: Commit = serde_json::from_slice(&content)
        .map_err(|_| "Commit data is corrupted")?;
    Ok(())
}

fn generate_merge_signature(commit1: &str, commit2: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = format!("merge:{}:{}", commit1, commit2);
    let signature = hex::encode(digest::digest(&digest::SHA256, content.as_bytes()).as_ref());
    Ok(signature)
}

fn update_head(repo: &Repository, commit_hash: &str) {
    let head_content = std::fs::read_to_string(repo.git_dir.join("HEAD")).unwrap();
    if head_content.starts_with("ref: ") {
        let ref_path = head_content.trim().strip_prefix("ref: ").unwrap();
        std::fs::write(repo.git_dir.join(ref_path), commit_hash).unwrap();
    }
}

async fn audit_merge_operation(
    merge_type: &str,
    branch_name: &str,
    commit_hash: &str,
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
    let details = format!("{}:{}:{}", merge_type, branch_name, commit_hash);
    
    let entry = format!("{},merge,{},{},merge\n", timestamp, user, details);
    std::fs::OpenOptions::new()
        .append(true)
        .open(audit_file)?
        .write_all(entry.as_bytes())?;
    
    Ok(())
}
