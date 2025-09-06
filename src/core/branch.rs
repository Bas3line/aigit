use crate::core::Repository;
use std::fs;

pub struct Branch {
    pub name: String,
    pub hash: Option<String>,
}

impl Branch {
    pub fn new(name: String, hash: Option<String>) -> Self {
        Self { name, hash }
    }

    pub fn create(repo: &Repository, name: &str, start_point: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let branch_path = repo.heads_dir().join(name);
        
        if branch_path.exists() {
            return Err(format!("Branch '{}' already exists", name).into());
        }

        let commit_hash = match start_point {
            Some(hash) => hash.to_string(),
            None => Self::get_current_commit(repo).unwrap_or_default(),
        };

        fs::write(branch_path, commit_hash)?;
        Ok(())
    }

    pub fn delete(repo: &Repository, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let branch_path = repo.heads_dir().join(name);
        
        if !branch_path.exists() {
            return Err(format!("Branch '{}' does not exist", name).into());
        }

        let current_branch = Self::get_current_branch(repo);
        if current_branch.as_deref() == Some(name) {
            return Err("Cannot delete the currently checked out branch".into());
        }

        fs::remove_file(branch_path)?;
        Ok(())
    }

    pub fn list(repo: &Repository) -> Result<Vec<Branch>, Box<dyn std::error::Error>> {
        let heads_dir = repo.heads_dir();
        let mut branches = Vec::new();

        if !heads_dir.exists() {
            return Ok(branches);
        }

        for entry in fs::read_dir(&heads_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                let hash = fs::read_to_string(entry.path()).ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                
                branches.push(Branch::new(name.to_string(), hash));
            }
        }

        branches.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(branches)
    }

    pub fn get_current_branch(repo: &Repository) -> Option<String> {
        let head_content = fs::read_to_string(repo.git_dir.join("HEAD")).ok()?;
        
        if head_content.starts_with("ref: refs/heads/") {
            head_content
                .strip_prefix("ref: refs/heads/")
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    }

    pub fn get_current_commit(repo: &Repository) -> Option<String> {
        let head_content = fs::read_to_string(repo.git_dir.join("HEAD")).ok()?;
        
        if head_content.starts_with("ref: refs/heads/") {
            let branch_name = head_content.strip_prefix("ref: refs/heads/")?.trim();
            let branch_path = repo.heads_dir().join(branch_name);
            fs::read_to_string(branch_path).ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            Some(head_content.trim().to_string())
        }
    }

    pub fn checkout(repo: &Repository, target: &str) -> Result<(), Box<dyn std::error::Error>> {
        let branch_path = repo.heads_dir().join(target);
        
        if branch_path.exists() {
            let new_head = format!("ref: refs/heads/{}", target);
            fs::write(repo.git_dir.join("HEAD"), new_head)?;
        } else if target.len() >= 4 && target.chars().all(|c| c.is_ascii_hexdigit()) {
            fs::write(repo.git_dir.join("HEAD"), target)?;
        } else {
            return Err(format!("Branch or commit '{}' not found", target).into());
        }

        Ok(())
    }
}
