use crate::core::{Repository, Index, Object, ObjectType};
use crate::utils::ignore::GitIgnore;
use walkdir::WalkDir;
use std::path::Path;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use ring::digest;
use std::io::Write;

pub async fn run(files: Vec<String>, all: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::new(".aigit").ok_or("Not in a repository")?;
    let mut index = Index::load(&repo)?;
    let ignore = GitIgnore::new(&repo.path);
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    pb.set_message("Scanning and adding files...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut added_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    if all || files.contains(&".".to_string()) {
        for entry in WalkDir::new(".")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !e.path().starts_with(".aigit"))
        {
            let path = entry.path();
            
            if ignore.is_ignored(path) {
                skipped_count += 1;
                continue;
            }
            
            if is_secure_file(path)? {
                match add_file_to_index(&mut index, &repo, path.to_str().unwrap()).await {
                    Ok(true) => added_count += 1,
                    Ok(false) => skipped_count += 1,
                    Err(_) => error_count += 1,
                }
            } else {
                skipped_count += 1;
            }
        }
    } else {
        for file in files {
            if !Path::new(&file).exists() {
                pb.finish_and_clear();
                println!("{} {}", "File not found:".red(), file);
                return Err("File not found".into());
            }
            
            if ignore.is_ignored(&file) {
                println!("{} {} (ignored)", "Skipping".yellow(), file);
                skipped_count += 1;
                continue;
            }
            
            if is_secure_file(Path::new(&file))? {
                match add_file_to_index(&mut index, &repo, &file).await {
                    Ok(true) => added_count += 1,
                    Ok(false) => skipped_count += 1,
                    Err(_) => error_count += 1,
                }
            } else {
                println!("{} {} (security check failed)", "Skipping".yellow(), file);
                skipped_count += 1;
            }
        }
    }

    index.save(&repo)?;
    pb.finish_and_clear();
    
    if added_count > 0 {
        println!("{} {} files to staging area", "Added".green(), added_count.to_string().bright_yellow());
    }
    if skipped_count > 0 {
        println!("{} {} files", "Skipped".yellow(), skipped_count);
    }
    if error_count > 0 {
        println!("{} {} files", "Errors".red(), error_count);
    }
    
    if added_count == 0 && skipped_count == 0 && error_count == 0 {
        println!("{}", "No files to add".yellow());
    }
    
    audit_add_operation(added_count, skipped_count, error_count).await?;
    Ok(())
}

async fn add_file_to_index(index: &mut Index, repo: &Repository, file_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let content = std::fs::read(file_path)?;
    
    if content.len() > 104_857_600 {
        println!("{} {} (file too large)", "Skipping".yellow(), file_path);
        return Ok(false);
    }
    
    scan_file_content(&content, file_path)?;
    
    let blob_hash = Object::create(repo, ObjectType::Blob, &content)?;
    let mode = get_file_mode(file_path);
    let size = content.len() as u64;
    let checksum = hex::encode(digest::digest(&digest::SHA256, &content).as_ref());
    
    index.add_entry_secure(file_path.to_string(), blob_hash, mode, size, checksum);
    
    Ok(true)
}

fn is_secure_file(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let blocked_extensions = [
        ".exe", ".dll", ".bat", ".cmd", ".com", ".pif", ".scr", ".vbs", ".js", ".jar",
        ".app", ".dmg", ".pkg", ".deb", ".rpm", ".msi", ".run", ".bin", ".sh", ".ps1"
    ];
    
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            let ext_lower = ext_str.to_lowercase();
            if blocked_extensions.contains(&ext_lower.as_str()) {
                return Ok(false);
            }
        }
    }
    
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let suspicious_names = [
        "id_rsa", "id_dsa", "id_ed25519", "id_ecdsa", ".env", ".env.local",
        "config.ini", "database.sqlite", "credentials", "secrets", "private.key"
    ];
    
    for name in &suspicious_names {
        if filename.eq_ignore_ascii_case(name) {
            println!("{} {} (potentially sensitive file)", "Warning".yellow(), path.display());
        }
    }
    
    Ok(true)
}

fn scan_file_content(content: &[u8], file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if content.is_empty() {
        return Ok(());
    }
    
    let text_content = String::from_utf8_lossy(content);
    let suspicious_patterns = [
        r"-----BEGIN (RSA |DSA |EC |OPENSSH |PRIVATE )?PRIVATE KEY-----",
        r#"password\s*=\s*['\"][^'"]{6,}['"]"#,
        r#"secret\s*=\s*['\"][^'"]{10,}['"]"#,
        r#"api[_-]?key\s*=\s*['\"][^'"]{20,}['"]"#,
        r#"token\s*=\s*['\"][^'"]{20,}['"]"#,
        r#"AKIA[0-9A-Z]{16}"#,
        r#"sk_live_[0-9a-zA-Z]{24}"#,
    ];
    
    for pattern in &suspicious_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(&text_content) {
                println!("{} {} contains potentially sensitive data", "Warning".yellow(), file_path);
                break;
            }
        }
    }
    
    Ok(())
}

fn get_file_mode(file_path: &str) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(file_path) {
            Ok(metadata) => {
                let mode = metadata.permissions().mode();
                if mode & 0o111 != 0 {
                    "100755".to_string()
                } else {
                    "100644".to_string()
                }
            },
            Err(_) => "100644".to_string(),
        }
    }
    #[cfg(not(unix))]
    {
        "100644".to_string()
    }
}

async fn audit_add_operation(added: usize, skipped: usize, errors: usize) -> Result<(), Box<dyn std::error::Error>> {
    let audit_file = std::path::PathBuf::from(".aigit/logs/audit.log");
    if !audit_file.exists() {
        return Ok(());
    }
    
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = whoami::username();
    let details = format!("added:{},skipped:{},errors:{}", added, skipped, errors);
    
    let entry = format!("{},add,{},{},files\n", timestamp, user, details);
    std::fs::OpenOptions::new()
        .append(true)
        .open(audit_file)?
        .write_all(entry.as_bytes())?;
    
    Ok(())
}
