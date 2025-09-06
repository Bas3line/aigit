use crate::core::{Repository, Index};
use crate::ai::gemini::GeminiClient;
use crate::utils::diff::get_staged_diff;
use crate::utils::analyzer::analyze_diff_complexity;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn run(full: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let index = Index::load(&repo)?;

    if index.entries.is_empty() {
        println!("{}", "No changes staged for review".yellow());
        return Ok(());
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("AI analyzing staged changes...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let diff_content = get_staged_diff(&repo, &index).await;
    let complexity_score = analyze_diff_complexity(&diff_content).await;
    
    let gemini = GeminiClient::new();

    match gemini.comprehensive_review(&diff_content, full).await {
        Ok(review) => {
            pb.finish_and_clear();
            
            print_review_header(&index, complexity_score);
            println!("{}", review);
            
            if full {
                println!("\n{}", "Generating additional insights...".yellow());
                
                if let Ok(suggestions) = gemini.suggest_improvements(&diff_content).await {
                    println!("\n{}", "=== Improvement Suggestions ===".green().bold());
                    println!("{}", suggestions);
                }
                
                if let Ok(security_analysis) = analyze_security_implications(&diff_content, &gemini).await {
                    println!("\n{}", "=== Security Analysis ===".red().bold());
                    println!("{}", security_analysis);
                }
                
                if let Ok(performance_analysis) = analyze_performance_implications(&diff_content, &gemini).await {
                    println!("\n{}", "=== Performance Analysis ===".blue().bold());
                    println!("{}", performance_analysis);
                }
            }
            
            print_review_summary(&index, complexity_score);
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Review failed: {}", e).into());
        }
    }
    
    Ok(())
}

fn print_review_header(index: &Index, complexity_score: f32) {
    println!("{}", "=== AI Code Review ===".cyan().bold());
    println!("Files staged: {}", index.entries.len().to_string().bright_yellow());
    
    let complexity_level = match complexity_score {
        score if score < 2.0 => ("Low", "green"),
        score if score < 5.0 => ("Medium", "yellow"),
        score if score < 10.0 => ("High", "orange"),
        _ => ("Very High", "red"),
    };
    
    println!("Complexity: {} ({:.1})", 
            complexity_level.0.color(complexity_level.1), 
            complexity_score);
    
    if index.has_conflicts() {
        println!("{} Unresolved conflicts detected", "⚠️".red());
    }
    
    println!("{}", "─".repeat(60).bright_black());
}

fn print_review_summary(index: &Index, complexity_score: f32) {
    println!("\n{}", "=== Review Summary ===".cyan().bold());
    
    let total_size: u64 = index.metadata.values().map(|entry| entry.size).sum();
    println!("Total size: {} bytes", total_size.to_string().bright_blue());
    
    if complexity_score > 8.0 {
        println!("{} Consider breaking this into smaller commits", "💡".yellow());
    }
    
    if index.entries.len() > 20 {
        println!("{} Large number of files - consider reviewing in batches", "📋".yellow());
    }
    
    println!("{}", "Review completed successfully".green());
}

async fn analyze_security_implications(
    diff_content: &str,
    gemini: &GeminiClient
) -> Result<String, Box<dyn std::error::Error>> {
    let security_prompt = format!(
        "Analyze these code changes for potential security vulnerabilities, \
        including but not limited to: SQL injection, XSS, authentication bypasses, \
        data exposure, input validation issues, and unsafe operations:\n\n{}",
        diff_content.chars().take(3000).collect::<String>()
    );
    
    gemini.generate_text(&security_prompt).await
}

async fn analyze_performance_implications(
    diff_content: &str,
    gemini: &GeminiClient
) -> Result<String, Box<dyn std::error::Error>> {
    let performance_prompt = format!(
        "Analyze these code changes for performance implications, \
        including: algorithmic complexity, memory usage, I/O operations, \
        database queries, caching opportunities, and bottlenecks:\n\n{}",
        diff_content.chars().take(3000).collect::<String>()
    );
    
    gemini.generate_text(&performance_prompt).await
}
