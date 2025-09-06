use crate::core::Repository;
use std::fs;
use std::collections::HashMap;

pub struct Refs {
    pub heads: HashMap<String, String>,
    pub tags: HashMap<String, String>,
}

impl Refs {
    pub fn load(repo: &Repository) -> Result<Self, Box<dyn std::error::Error>> {
        let mut refs = Refs {
            heads: HashMap::new(),
            tags: HashMap::new(),
        };

        let heads_dir = repo.heads_dir();
        if heads_dir.exists() {
            for entry in fs::read_dir(&heads_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(hash) = fs::read_to_string(entry.path()) {
                        let hash = hash.trim();
                        if !hash.is_empty() {
                            refs.heads.insert(name.to_string(), hash.to_string());
                        }
                    }
                }
            }
        }

        let tags_dir = repo.tags_dir();
        if tags_dir.exists() {
            for entry in fs::read_dir(&tags_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(hash) = fs::read_to_string(entry.path()) {
                        let hash = hash.trim();
                        if !hash.is_empty() {
                            refs.tags.insert(name.to_string(), hash.to_string());
                        }
                    }
                }
            }
        }

        Ok(refs)
    }

    pub fn get_head(&self, name: &str) -> Option<&String> {
        self.heads.get(name)
    }

    pub fn get_tag(&self, name: &str) -> Option<&String> {
        self.tags.get(name)
    }

    pub fn resolve(&self, name: &str) -> Option<&String> {
        self.get_head(name).or_else(|| self.get_tag(name))
    }

    pub fn create_tag(&mut self, repo: &Repository, name: &str, commit_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tag_path = repo.tags_dir().join(name);
        fs::write(&tag_path, commit_hash)?;
        self.tags.insert(name.to_string(), commit_hash.to_string());
        Ok(())
    }

    pub fn delete_tag(&mut self, repo: &Repository, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tag_path = repo.tags_dir().join(name);
        if tag_path.exists() {
            fs::remove_file(&tag_path)?;
            self.tags.remove(name);
        }
        Ok(())
    }
}
