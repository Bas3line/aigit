use std::fs;
use std::path::Path;
use colored::*;
use ring::digest;
use rand::RngCore;

pub async fn run(bare: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = if bare { "." } else { ".aigit" };
    
    if Path::new(repo_dir).join("HEAD").exists() {
        println!("{}", "Repository already initialized".yellow());
        return Ok(());
    }

    create_secure_repo_structure(repo_dir, bare)?;
    initialize_security_settings(repo_dir)?;
    
    let msg = if bare {
        "Initialized secure AI-powered bare repository"
    } else {
        "Initialized secure AI-powered repository in .aigit/"
    };
    
    println!("{}", msg.green());
    Ok(())
}

fn create_secure_repo_structure(repo_dir: &str, bare: bool) -> Result<(), Box<dyn std::error::Error>> {
    let dirs = vec![
        format!("{}/objects", repo_dir),
        format!("{}/refs/heads", repo_dir),
        format!("{}/refs/tags", repo_dir),
        format!("{}/hooks", repo_dir),
        format!("{}/security", repo_dir),
        format!("{}/logs", repo_dir),
        format!("{}/info", repo_dir),
    ];

    for dir in dirs {
        fs::create_dir_all(&dir)?;
        set_secure_permissions(&dir)?;
    }
    
    let repo_id = generate_secure_repo_id();
    fs::write(format!("{}/HEAD", repo_dir), "ref: refs/heads/main\n")?;
    fs::write(format!("{}/description", repo_dir), "Secure AI repository\n")?;
    fs::write(format!("{}/config", repo_dir), create_secure_config(bare))?;
    fs::write(format!("{}/info/repo-id", repo_dir), repo_id)?;
    fs::write(format!("{}/info/exclude", repo_dir), create_default_excludes())?;
    
    if !bare {
        fs::write(format!("{}/index", repo_dir), "")?;
        fs::write(".gitignore", create_default_gitignore())?;
    }

    create_security_hooks(repo_dir)?;
    
    Ok(())
}

fn set_secure_permissions(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

fn generate_secure_repo_id() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    hex::encode(digest::digest(&digest::SHA256, &bytes).as_ref())
}

fn create_secure_config(bare: bool) -> String {
    format!(
        r#"[core]
    repositoryformatversion = 0
    filemode = true
    bare = {}
    logallrefupdates = true
    precomposeunicode = true
    protectHFS = true
    protectNTFS = true
    ignorecase = false
    trustctime = false
    checkStat = minimal
    autocrlf = false
    safecrlf = true
    quotepath = false

[security]
    enabled = true
    requireSignature = true
    auditLog = true
    encryptObjects = false
    hashAlgorithm = sha256
    compressionLevel = 6

[ai]
    enabled = true
    model = gemini-pro
    maxTokens = 2048
    temperature = 0.7
    requireReview = false
    autoCommitMessage = true

[user]
    signingkey = ""
    
[commit]
    gpgsign = false
    template = ""
"#,
        bare
    )
}

fn create_default_excludes() -> &'static str {
    r#"*.o
*.a
*.so
*~
*.swp
*.tmp
.DS_Store
Thumbs.db
*.log
*.bak
*.orig
core
.#*
#*#"#
}

fn create_default_gitignore() -> &'static str {
    r#"target/
*.tmp
*.log
.env
.DS_Store
node_modules/
*.swp
*.swo
__pycache__/
*.pyc
.pytest_cache/
.coverage
*.orig
*.rej
.idea/
.vscode/
*.iml
dist/
build/
.cache/
*.lock
!Cargo.lock"#
}

fn create_security_hooks(repo_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hooks = vec![
        ("pre-commit", PRE_COMMIT_HOOK),
        ("commit-msg", COMMIT_MSG_HOOK),
        ("pre-receive", PRE_RECEIVE_HOOK),
        ("post-receive", POST_RECEIVE_HOOK),
    ];

    for (hook_name, hook_content) in hooks {
        let hook_path = format!("{}/hooks/{}", repo_dir, hook_name);
        fs::write(&hook_path, hook_content)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)?;
        }
    }

    Ok(())
}

const PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
if [ -f ".aigit/security/pre-commit-checks" ]; then
    exec ".aigit/security/pre-commit-checks"
fi
exit 0
"#;

const COMMIT_MSG_HOOK: &str = r#"#!/bin/sh
if [ -f ".aigit/security/commit-msg-checks" ]; then
    exec ".aigit/security/commit-msg-checks" "$1"
fi
exit 0
"#;

const PRE_RECEIVE_HOOK: &str = r#"#!/bin/sh
if [ -f ".aigit/security/pre-receive-checks" ]; then
    exec ".aigit/security/pre-receive-checks"
fi
exit 0
"#;

const POST_RECEIVE_HOOK: &str = r#"#!/bin/sh
if [ -f ".aigit/security/post-receive-hooks" ]; then
    exec ".aigit/security/post-receive-hooks"
fi
exit 0
"#;

fn initialize_security_settings(repo_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let security_config = r#"{
    "audit_log": true,
    "require_signature": false,
    "encrypt_objects": false,
    "hash_algorithm": "sha256",
    "max_file_size": 104857600,
    "blocked_extensions": [".exe", ".dll", ".bat", ".cmd", ".com", ".pif", ".scr"],
    "scan_content": true,
    "rate_limit": {
        "commits_per_hour": 100,
        "size_limit_mb": 100
    }
}"#;

    fs::write(format!("{}/security/config.json", repo_dir), security_config)?;
    
    let audit_log_header = r#"timestamp,action,user,details,hash
"#;
    fs::write(format!("{}/logs/audit.log", repo_dir), audit_log_header)?;
    
    Ok(())
}
