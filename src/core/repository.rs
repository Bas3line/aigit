use std::path::{Path, PathBuf};
use thiserror::Error;
use ring::digest;

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Not a valid aigit repository")]
    NotARepo,
    #[error("Repository already exists")]
    AlreadyExists,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Repository corrupted: {0}")]
    Corrupted(String),
}

pub struct Repository {
    pub path: PathBuf,
    pub git_dir: PathBuf,
    pub repo_id: String,
}

impl Repository {
    pub fn new<P: AsRef<Path>>(git_dir: P) -> Option<Self> {
        let git_dir = git_dir.as_ref().to_path_buf();
        
        if !Self::is_valid_repo(&git_dir) {
            return None;
        }
        
        let path = if git_dir.file_name() == Some(std::ffi::OsStr::new(".aigit")) {
            git_dir.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };
        
        let path = if path.as_os_str().is_empty() {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            path
        };
        
        let repo_id = Self::load_repo_id(&git_dir).unwrap_or_else(|| "unknown".to_string());
        
        Some(Repository { 
            path, 
            git_dir, 
            repo_id 
        })
    }

    pub fn init<P: AsRef<Path>>(path: P, bare: bool) -> Result<Self, RepoError> {
        let path = path.as_ref();
        let git_dir = if bare {
            path.to_path_buf()
        } else {
            path.join(".aigit")
        };

        if git_dir.join("HEAD").exists() {
            return Err(RepoError::AlreadyExists);
        }

        Self::create_repo_structure(&git_dir, bare)?;
        let repo_id = Self::generate_repo_id(&git_dir);
        
        let work_dir = if bare { git_dir.clone() } else { path.to_path_buf() };
        
        Ok(Repository {
            path: work_dir,
            git_dir,
            repo_id,
        })
    }

    fn is_valid_repo(git_dir: &Path) -> bool {
        git_dir.join("HEAD").exists() && 
        git_dir.join("objects").exists() && 
        git_dir.join("refs").exists()
    }

    fn create_repo_structure(git_dir: &Path, bare: bool) -> Result<(), RepoError> {
        let dirs = vec![
            git_dir.join("objects"),
            git_dir.join("refs/heads"),
            git_dir.join("refs/tags"),
            git_dir.join("hooks"),
            git_dir.join("security"),
            git_dir.join("logs"),
            git_dir.join("info"),
        ];

        for dir in dirs {
            std::fs::create_dir_all(&dir)?;
            Self::set_secure_permissions(&dir)?;
        }

        let head_content = "ref: refs/heads/main\n";
        std::fs::write(git_dir.join("HEAD"), head_content)?;
        
        let config_content = Self::create_repo_config(bare);
        std::fs::write(git_dir.join("config"), config_content)?;
        
        let description = "AI-powered secure repository\n";
        std::fs::write(git_dir.join("description"), description)?;

        if !bare {
            std::fs::write(git_dir.join("index"), "")?;
        }

        Ok(())
    }

    fn create_repo_config(bare: bool) -> String {
        format!(
            r#"[core]
    repositoryformatversion = 0
    filemode = true
    bare = {}
    logallrefupdates = true
    ignorecase = false
    precomposeunicode = true
    protectHFS = true
    protectNTFS = true
    quotepath = false

[security]
    enabled = true
    auditLog = true
    requireSignature = false
    encryptObjects = false
    hashAlgorithm = sha256

[ai]
    enabled = true
    model = gemini-pro
    autoCommitMessage = true
    reviewRequired = false
"#,
            bare
        )
    }

    fn generate_repo_id(git_dir: &Path) -> String {
        let content = format!("{}{}", 
                             git_dir.to_string_lossy(), 
                             chrono::Utc::now().to_rfc3339());
        let digest = digest::digest(&digest::SHA256, content.as_bytes());
        let id = hex::encode(digest.as_ref())[..16].to_string();
        
        if let Err(_) = std::fs::write(git_dir.join("info/repo-id"), &id) {
            return "fallback".to_string();
        }
        
        id
    }

    fn load_repo_id(git_dir: &Path) -> Option<String> {
        std::fs::read_to_string(git_dir.join("info/repo-id"))
            .ok()
            .map(|s| s.trim().to_string())
    }

    fn set_secure_permissions(path: &Path) -> Result<(), RepoError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o750);
            std::fs::set_permissions(path, perms)?;
        }
        Ok(())
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.git_dir.join("objects")
    }

    pub fn refs_dir(&self) -> PathBuf {
        self.git_dir.join("refs")
    }

    pub fn heads_dir(&self) -> PathBuf {
        self.refs_dir().join("heads")
    }

    pub fn tags_dir(&self) -> PathBuf {
        self.refs_dir().join("tags")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.git_dir.join("logs")
    }

    pub fn security_dir(&self) -> PathBuf {
        self.git_dir.join("security")
    }

    pub fn is_bare(&self) -> bool {
        self.path == self.git_dir
    }

    pub fn verify_integrity(&self) -> Result<(), RepoError> {
        if !self.git_dir.join("HEAD").exists() {
            return Err(RepoError::Corrupted("Missing HEAD file".to_string()));
        }

        if !self.objects_dir().exists() {
            return Err(RepoError::Corrupted("Missing objects directory".to_string()));
        }

        if !self.refs_dir().exists() {
            return Err(RepoError::Corrupted("Missing refs directory".to_string()));
        }

        let expected_id = Self::generate_repo_id(&self.git_dir);
        if self.repo_id != "unknown" && self.repo_id != expected_id {
            return Err(RepoError::Corrupted("Repository ID mismatch".to_string()));
        }

        Ok(())
    }

    pub fn get_security_config(&self) -> Option<serde_json::Value> {
        let security_file = self.security_dir().join("config.json");
        if security_file.exists() {
            std::fs::read_to_string(security_file)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
        } else {
            None
        }
    }
}
