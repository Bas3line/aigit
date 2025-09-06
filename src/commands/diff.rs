use crate::core::{Repository, Index};
use crate::ai::gemini::GeminiClient;
use crate::utils::diff::{generate_diff, calculate_diff_stats};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn run(cached: bool, ai_explain: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    
    let diff_content = if cached {
        get_staged_diff(&repo).await?
    } else {
        get_working_diff(&repo).await?
    };

    if diff_content.is_empty() {
        println!("{}", "No changes found".yellow());
        return Ok(());
    }

    let (additions, deletions, modifications) = calculate_diff_stats(&diff_content).await;
    
    println!("{}", diff_content);
    
    print_diff_summary(additions, deletions, modifications, cached);

    if ai_explain {
        explain_changes_with_ai(&diff_content).await?;
    }
        
    
    Ok(())
}

async fn get_staged_diff(repo: &Repository) -> Result<String, Box<dyn std::error::Error>> {
    let index = Index::load(repo)?;
    generate_diff(repo, &index, true).await
}

async fn get_working_diff(repo: &Repository) -> Result<String, Box<dyn std::error::Error>> {
    let index = Index::load(repo)?;
    generate_diff(repo, &index, false).await
}

fn print_diff_summary(additions: usize, deletions: usize, modifications: usize, staged: bool) {
    let diff_type = if staged { "staged" } else { "working tree" };
    
    println!("\n{}", format!("=== {} changes ===", diff_type).cyan().bold());
    
    if additions > 0 {
        println!("{} {} lines added", "+".green(), additions.to_string().green());
    }
    if deletions > 0 {
        println!("{} {} lines deleted", "-".red(), deletions.to_string().red());
    }
    if modifications > 0 {
        println!("{} {} lines modified", "~".yellow(), modifications.to_string().yellow());
    }
    
    let total_changes = additions + deletions + modifications;
    if total_changes == 0 {
        println!("{}", "No changes detected".bright_black());
    } else {
        println!("{} {} total changes", "∑".bright_blue(), total_changes.to_string().bright_blue());
    }
}

async fn explain_changes_with_ai(diff_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("AI analyzing changes...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let gemini = GeminiClient::new();
    match gemini.explain_diff(diff_content).await {
        Ok(explanation) => {
            pb.finish_and_clear();
            println!("\n{}", "=== AI Explanation ===".cyan().bold());
            println!("{}", explanation);
        },
        Err(e) => {
            pb.finish_and_clear();
            println!("{} {}", "Failed to explain changes:".red(), e);
        }
    }
    
    Ok(())
}
