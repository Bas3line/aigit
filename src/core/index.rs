use crate::core::Repository;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use ring::digest;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct IndexEntry {
    pub hash: String,
    pub mode: String,
    pub size: u64,
    pub mtime: DateTime<Utc>,
    pub ctime: DateTime<Utc>,
    pub stage: u8,
    pub checksum: String,
    pub flags: u16,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Index {
    pub entries: HashMap<String, String>,
    pub metadata: HashMap<String, IndexEntry>,
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<String>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            metadata: HashMap::new(),
            version: 3,
            timestamp: Utc::now(),
            signature: None,
        }
    }

    pub fn load(repo: &Repository) -> Result<Self, Box<dyn std::error::Error>> {
        let index_path = repo.git_dir.join("index");
        
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            if content.trim().is_empty() {
                return Ok(Index::new());
            }
            let index: Index = serde_json::from_str(&content)
                .unwrap_or_else(|_| Index::new());
            
            index.verify_integrity()?;
            Ok(index)
        } else {
            Ok(Index::new())
        }
    }

    pub fn save(&mut self, repo: &Repository) -> Result<(), Box<dyn std::error::Error>> {
        self.timestamp = Utc::now();
        self.update_signature();
        
        let index_path = repo.git_dir.join("index");
        let content = serde_json::to_string_pretty(self)?;
        
        let temp_path = index_path.with_extension("tmp");
        std::fs::write(&temp_path, content)?;
        std::fs::rename(&temp_path, &index_path)?;
        
        self.set_index_permissions(&index_path)?;
        Ok(())
    }

    pub fn add_entry(&mut self, path: String, hash: String, mode: String) {
        let now = Utc::now();
        
        let metadata = if let Ok(file_metadata) = std::fs::metadata(&path) {
            let content = std::fs::read(&path).unwrap_or_default();
            let checksum = hex::encode(digest::digest(&digest::SHA256, &content).as_ref());
            
            IndexEntry {
                hash: hash.clone(),
                mode: mode.clone(),
                size: file_metadata.len(),
                mtime: now,
                ctime: now,
                stage: 0,
                checksum,
                flags: 0,
            }
        } else {
            IndexEntry {
                hash: hash.clone(),
                mode,
                size: 0,
                mtime: now,
                ctime: now,
                stage: 0,
                checksum: String::new(),
                flags: 0,
            }
        };
        
        self.entries.insert(path.clone(), hash);
        self.metadata.insert(path, metadata);
        self.timestamp = now;
    }

    pub fn add_entry_secure(&mut self, path: String, hash: String, mode: String, size: u64, checksum: String) {
        let now = Utc::now();
        
        let metadata = IndexEntry {
            hash: hash.clone(),
            mode,
            size,
            mtime: now,
            ctime: now,
            stage: 0,
            checksum,
            flags: 0,
        };
        
        self.entries.insert(path.clone(), hash);
        self.metadata.insert(path, metadata);
        self.timestamp = now;
    }

    pub fn remove_entry(&mut self, path: &str) {
        self.entries.remove(path);
        self.metadata.remove(path);
        self.timestamp = Utc::now();
    }

    pub fn clear(&mut self, _repo: &Repository) -> Result<(), Box<dyn std::error::Error>> {
        self.entries.clear();
        self.metadata.clear();
        self.timestamp = Utc::now();
        self.signature = None;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has_conflicts(&self) -> bool {
        self.metadata.values().any(|entry| entry.stage != 0)
    }

    pub fn get_conflicted_files(&self) -> Vec<String> {
        self.metadata
            .iter()
            .filter(|(_, entry)| entry.stage != 0)
            .map(|(path, _)| path.clone())
            .collect()
    }

    fn verify_integrity(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.version < 2 || self.version > 4 {
            return Err("Unsupported index version".into());
        }

        for (path, hash) in &self.entries {
            if hash.len() < 8 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("Invalid hash format for {}", path).into());
            }
            
            if !self.metadata.contains_key(path) {
                return Err(format!("Missing metadata for {}", path).into());
            }
        }

        for (path, entry) in &self.metadata {
            if !self.entries.contains_key(path) {
                return Err(format!("Orphaned metadata for {}", path).into());
            }
            
            if entry.stage > 3 {
                return Err(format!("Invalid stage number for {}", path).into());
            }
        }

        Ok(())
    }

    fn update_signature(&mut self) {
        let content = format!("{}{}{}",
                             self.entries.len(),
                             self.timestamp.to_rfc3339(),
                             self.version);
        let digest_result = digest::digest(&digest::SHA256, content.as_bytes());
        self.signature = Some(hex::encode(digest_result.as_ref())[..16].to_string());
    }

    fn set_index_permissions(&self, index_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(index_path)?.permissions();
            perms.set_mode(0o644);
            std::fs::set_permissions(index_path, perms)?;
        }
        Ok(())
    }
}
