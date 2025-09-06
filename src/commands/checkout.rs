use crate::core::{Repository, Branch};
use colored::*;
use std::path::Path;

pub async fn run(target: String, create: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo_path = Path::new(".aigit");
    if !repo_path.exists() {
        return Err("Not in an AIGIT repository".into());
    }

    let repo = Repository::new(".aigit")
        .ok_or("Failed to open repository")?;
    
    if create {
        // Create and checkout new branch
        println!("{} Creating and switching to branch '{}'", "✓".green(), target);
        Branch::create(&repo, &target, None)?;
        Branch::checkout(&repo, &target)?;
        println!("{} Switched to new branch '{}'", "✓".green(), target);
    } else {
        // Checkout existing branch or commit
        let branch_path = repo.heads_dir().join(&target);
        if branch_path.exists() {
            Branch::checkout(&repo, &target)?;
            println!("{} Switched to branch '{}'", "✓".green(), target);
        } else {
            // Try to checkout as commit hash
            if target.len() >= 4 && target.chars().all(|c| c.is_ascii_hexdigit()) {
                Branch::checkout(&repo, &target)?;
                println!("{} Switched to commit '{}'", "✓".green(), target);
            } else {
                return Err(format!("Branch '{}' does not exist. Use --create to create it.", target).into());
            }
        }
    }
    
    Ok(())
}