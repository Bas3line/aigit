use crate::core::Repository;
use crate::ai::gemini::GeminiClient;
use crate::utils::analyzer::analyze_codebase;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn commit() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Analyzing project context...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let context = analyze_codebase(&repo).await;
    let gemini = GeminiClient::new();

    match gemini.suggest_next_commit(&context).await {
        Ok(suggestion) => {
            pb.finish_and_clear();
            println!("{}", "AI Suggests Next Steps:".cyan().bold());
            println!("{}", suggestion);
            print_commit_best_practices();
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Failed to generate suggestion: {}", e).into());
        }
    }
    
    Ok(())
}

pub async fn branch() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Analyzing project for branch opportunities...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let context = analyze_codebase(&repo).await;
    let gemini = GeminiClient::new();

    match gemini.suggest_branch_name(&context).await {
        Ok(suggestions) => {
            pb.finish_and_clear();
            println!("{}", "AI Suggested Branch Names:".cyan().bold());
            
            for (i, name) in suggestions.iter().enumerate() {
                let category = categorize_branch(&name);
                let icon = get_branch_icon(&category);
                println!("{}. {} {} {}", 
                        i + 1, 
                        icon,
                        name.bright_green(), 
                        format!("({})", category).bright_black());
            }
            
            print_branch_best_practices();
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Failed to generate suggestions: {}", e).into());
        }
    }
    
    Ok(())
}

pub async fn refactor() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Scanning codebase for refactoring opportunities...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let context = analyze_codebase(&repo).await;
    let gemini = GeminiClient::new();

    match gemini.suggest_refactoring(&context).await {
        Ok(suggestions) => {
            pb.finish_and_clear();
            println!("{}", "Refactoring Opportunities:".cyan().bold());
            println!("{}", suggestions);
            print_refactoring_guidelines();
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Failed to analyze code: {}", e).into());
        }
    }
    
    Ok(())
}

pub async fn tests() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Analyzing test coverage and opportunities...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let context = analyze_codebase(&repo).await;
    let gemini = GeminiClient::new();

    match gemini.suggest_tests(&context).await {
        Ok(suggestions) => {
            pb.finish_and_clear();
            println!("{}", "Testing Suggestions:".cyan().bold());
            println!("{}", suggestions);
            print_testing_best_practices();
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Failed to analyze testing needs: {}", e).into());
        }
    }
    
    Ok(())
}

pub async fn cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Identifying cleanup opportunities...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let context = analyze_codebase(&repo).await;
    let gemini = GeminiClient::new();
    
    let cleanup_prompt = format!(
        "Analyze this codebase and suggest cleanup tasks like removing dead code, \
        updating dependencies, fixing linting issues, improving documentation, \
        and removing technical debt:\n\n{}",
        context
    );

    match gemini.generate_text(&cleanup_prompt).await {
        Ok(suggestions) => {
            pb.finish_and_clear();
            println!("{}", "Cleanup Suggestions:".cyan().bold());
            println!("{}", suggestions);
            print_cleanup_checklist();
        },
        Err(e) => {
            pb.finish_and_clear();
            return Err(format!("Failed to generate cleanup suggestions: {}", e).into());
        }
    }
    
    Ok(())
}

fn categorize_branch(name: &str) -> &str {
    if name.starts_with("feature/") || name.starts_with("feat/") { "feature" }
    else if name.starts_with("bugfix/") || name.starts_with("fix/") { "bugfix" }
    else if name.starts_with("hotfix/") { "hotfix" }
    else if name.starts_with("refactor/") || name.starts_with("refact/") { "refactor" }
    else if name.starts_with("chore/") { "chore" }
    else if name.starts_with("docs/") { "documentation" }
    else if name.starts_with("test/") { "testing" }
    else if name.starts_with("perf/") { "performance" }
    else if name.starts_with("style/") { "style" }
    else { "general" }
}

fn get_branch_icon(category: &str) -> &str {
    match category {
        "feature" => "✨",
        "bugfix" => "🐛",
        "hotfix" => "🚨",
        "refactor" => "♻️",
        "chore" => "🔧",
        "documentation" => "📝",
        "testing" => "🧪",
        "performance" => "⚡",
        "style" => "💄",
        _ => "🔀",
    }
}

fn print_commit_best_practices() {
    println!("\n{}", "Commit Best Practices:".yellow().bold());
    println!("• {} Use conventional commit format: feat/fix/docs/style/refactor/test/chore", "📋".cyan());
    println!("• {} Keep first line under 50 characters", "📏".cyan());
    println!("• {} Use imperative mood: 'Add feature' not 'Added feature'", "✍️".cyan());
    println!("• {} Separate subject from body with blank line", "📄".cyan());
    println!("• {} Include why, not just what", "🤔".cyan());
}

fn print_branch_best_practices() {
    println!("\n{}", "Branch Naming Best Practices:".yellow().bold());
    println!("• {} Use descriptive names: feature/user-authentication", "📝".cyan());
    println!("• {} Include ticket number: bugfix/PROJ-123-login-error", "🎫".cyan());
    println!("• {} Use lowercase with hyphens", "🔤".cyan());
    println!("• {} Keep it concise but clear", "✂️".cyan());
    println!("• {} Delete after merging", "🗑️".cyan());
}

fn print_refactoring_guidelines() {
    println!("\n{}", "Refactoring Guidelines:".yellow().bold());
    println!("• {} Make small, incremental changes", "🔄".cyan());
    println!("• {} Ensure tests pass before and after", "✅".cyan());
    println!("• {} Document breaking changes", "📋".cyan());
    println!("• {} Consider backward compatibility", "⬅️".cyan());
    println!("• {} Review performance impact", "📊".cyan());
}

fn print_testing_best_practices() {
    println!("\n{}", "Testing Best Practices:".yellow().bold());
    println!("• {} Follow Arrange-Act-Assert pattern", "🏗️".cyan());
    println!("• {} Test edge cases and error conditions", "🧪".cyan());
    println!("• {} Use descriptive test names", "📝".cyan());
    println!("• {} Keep tests fast and isolated", "⚡".cyan());
    println!("• {} Aim for good coverage, not 100%", "📊".cyan());
}

fn print_cleanup_checklist() {
    println!("\n{}", "Cleanup Checklist:".yellow().bold());
    println!("• {} Remove unused imports and variables", "🧹".cyan());
    println!("• {} Update outdated comments", "💬".cyan());
    println!("• {} Fix linting warnings", "🔧".cyan());
    println!("• {} Remove console.log/print statements", "🗑️".cyan());
    println!("• {} Update dependencies", "📦".cyan());
    println!("• {} Remove dead code", "💀".cyan());
}
