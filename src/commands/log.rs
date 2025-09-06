use crate::core::{Repository, Commit, Object, Config};
use crate::ai::gemini::GeminiClient;
use colored::*;
use chrono::{DateTime, Local, TimeZone};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;

pub async fn run(oneline: bool, graph: bool, ai_summary: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let config = Config::load_repo(&repo).unwrap_or_else(|_| Config::load_global().unwrap_or_default());
    let mut commits = Vec::new();
    
    if let Some(head_hash) = get_head_commit(&repo) {
        collect_commits(&repo, &head_hash, &mut commits, &mut HashMap::new()).await?;
    }

    if commits.is_empty() {
        println!("{}", "No commits found".yellow());
        return Ok(());
    }

    if ai_summary && commits.len() > 1 {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
        pb.set_message("AI analyzing commit history...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let gemini = GeminiClient::new();
        let commit_messages: Vec<String> = commits.iter()
            .take(20)
            .map(|(_, commit)| commit.message.clone())
            .collect();
        let summary_prompt = format!("Summarize this commit history and identify patterns:\n{}", 
                                   commit_messages.join("\n---\n"));
        
        match gemini.generate_text(&summary_prompt).await {
            Ok(summary) => {
                pb.finish_and_clear();
                println!("{}", "AI Summary of Recent Changes:".cyan().bold());
                println!("{}\n", summary);
                println!("{}", "─".repeat(80).bright_black());
            },
            Err(_) => {
                pb.finish_and_clear();
                println!("{}", "Failed to generate AI summary".red());
            }
        }
    }

    let max_display = if oneline { 50 } else { 25 };
    let displayed_commits = commits.iter().take(max_display);

    for (i, (hash, commit)) in displayed_commits.enumerate() {
        if oneline {
            print_oneline_commit(hash, commit, i == 0);
        } else {
            print_full_commit(hash, commit, graph, i == 0, &config);
        }
    }

    if commits.len() > max_display {
        println!("\n{} ({} more commits)", 
                "...".bright_black(), 
                (commits.len() - max_display).to_string().bright_yellow());
        println!("{}", "Use 'aigit log --oneline' for more compact view".bright_black());
    }

    print_log_statistics(&commits);
    
    Ok(())
}

fn print_oneline_commit(hash: &str, commit: &Commit, is_head: bool) {
    let prefix = if is_head { "* " } else { "  " };
    let hash_color = if is_head { hash[..8].bright_yellow() } else { hash[..8].yellow() };
    let message_color = if is_head { commit.short_message().bright_white() } else { commit.short_message().white() };
    
    println!("{}{} {}", prefix, hash_color, message_color);
}

fn print_full_commit(hash: &str, commit: &Commit, graph: bool, is_head: bool, config: &Config) {
    let prefix = if graph { 
        if is_head { "* " } else { "| " }
    } else { 
        "" 
    };
    
    let hash_display = if is_head { hash.bright_yellow() } else { hash.yellow() };
    
    println!("{}{} {}", prefix, "commit".yellow(), hash_display);
    
    if commit.is_merge() {
        println!("{}Merge: {} {}", 
                "    ", 
                commit.parents.get(0).map(|h| &h[..8]).unwrap_or("unknown").bright_blue(),
                commit.parents.get(1).map(|h| &h[..8]).unwrap_or("unknown").bright_blue());
    }
    
    println!("{}Author: {} <{}>", 
            "    ", 
            commit.author.name.bright_white(), 
            commit.author.email.cyan());
    
    let local_time: DateTime<Local> = Local.timestamp_opt(commit.author.timestamp.timestamp(), 0)
        .single()
        .unwrap_or_else(|| Local::now());
    
    println!("{}Date:   {}", 
            "    ", 
            local_time.format("%a %b %d %H:%M:%S %Y %z"));
    
    if let Some(signature) = &commit.signature {
        if config.get("security.requireSignature").map(|v| v == "true").unwrap_or(false) {
            println!("{}Signature: {} ✓", "    ", signature.chars().take(16).collect::<String>().bright_green());
        }
    }
    
    println!();
    for line in commit.message.lines() {
        if line.trim().is_empty() {
            println!();
        } else {
            println!("    {}", line);
        }
    }
    println!();
}

use std::pin::Pin;
use std::future::Future;

fn collect_commits<'a>(
    repo: &'a Repository, 
    start_hash: &'a str, 
    commits: &'a mut Vec<(String, Commit)>,
    visited: &'a mut HashMap<String, bool>
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async move {
        if visited.contains_key(start_hash) {
            return Ok(());
        }
        
        visited.insert(start_hash.to_string(), true);
        
        let content = Object::read(repo, start_hash)?;
        let commit: Commit = serde_json::from_slice(&content)?;
        
        commits.push((start_hash.to_string(), commit.clone()));
        
        for parent_hash in &commit.parents {
            if !parent_hash.is_empty() && !visited.contains_key(parent_hash) {
                collect_commits(repo, parent_hash, commits, visited).await?;
            }
        }
        
        Ok(())
    })
}

fn get_head_commit(repo: &Repository) -> Option<String> {
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
        .filter(|s| !s.is_empty() && s.len() >= 8)
}

fn print_log_statistics(commits: &[(String, Commit)]) {
    if commits.is_empty() {
        return;
    }
    
    let mut authors = HashMap::new();
    let mut total_lines = 0;
    
    for (_, commit) in commits {
        *authors.entry(commit.author.name.clone()).or_insert(0) += 1;
        total_lines += commit.message.lines().count();
    }
    
    println!("{}", "─".repeat(80).bright_black());
    println!("{}", "Repository Statistics:".cyan().bold());
    println!("Total commits: {}", commits.len().to_string().bright_yellow());
    println!("Average message length: {} lines", (total_lines / commits.len()).to_string().bright_blue());
    
    if authors.len() > 1 {
        println!("\nTop contributors:");
        let mut sorted_authors: Vec<_> = authors.iter().collect();
        sorted_authors.sort_by(|a, b| b.1.cmp(a.1));
        
        for (author, count) in sorted_authors.iter().take(5) {
            println!("  {} - {} commits", author.bright_white(), count.to_string().bright_green());
        }
    }
}
