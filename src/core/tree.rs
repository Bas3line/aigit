use crate::core::{Repository, Index, Object, ObjectType};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub name: String,
    pub hash: String,
    pub entry_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, mode: String, name: String, hash: String, entry_type: String) {
        let entry = TreeEntry {
            mode,
            name,
            hash,
            entry_type,
        };
        self.entries.push(entry);
        self.entries.sort_by(|a, b| a.name.cmp(&b.name));
    }

    pub fn create_from_index(repo: &Repository, index: &Index) -> Result<String, Box<dyn std::error::Error>> {
        let mut tree = Tree::new();
        let mut directories = HashMap::new();

        for (path, hash) in &index.entries {
            let parts: Vec<&str> = path.split('/').collect();
            
            if parts.len() == 1 {
                let mode = index.metadata.get(path)
                    .map(|m| m.mode.clone())
                    .unwrap_or_else(|| "100644".to_string());
                tree.add_entry(mode, path.clone(), hash.clone(), "blob".to_string());
            } else {
                let dir = parts[0];
                if !directories.contains_key(dir) {
                    directories.insert(dir.to_string(), Vec::new());
                }
                
                let remaining_path = parts[1..].join("/");
                directories.get_mut(dir).unwrap().push((remaining_path, hash.clone()));
            }
        }

        for (dir_name, files) in directories {
            let mut subtree = Tree::new();
            for (file_path, file_hash) in files {
                subtree.add_entry("100644".to_string(), file_path, file_hash, "blob".to_string());
            }
            
            let subtree_content = serde_json::to_string(&subtree)?;
            let subtree_hash = Object::create(repo, ObjectType::Tree, subtree_content.as_bytes())?;
            tree.add_entry("040000".to_string(), dir_name, subtree_hash, "tree".to_string());
        }

        let tree_content = serde_json::to_string(&tree)?;
        Object::create(repo, ObjectType::Tree, tree_content.as_bytes())
    }

    pub fn from_hash(repo: &Repository, hash: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = Object::read(repo, hash)?;
        Ok(serde_json::from_slice(&content)?)
    }

    pub fn get_entry(&self, name: &str) -> Option<&TreeEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub fn list_files(&self, repo: &Repository, prefix: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        
        for entry in &self.entries {
            let full_path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{}/{}", prefix, entry.name)
            };
            
            if entry.entry_type == "blob" {
                files.push(full_path);
            } else if entry.entry_type == "tree" {
                let subtree = Tree::from_hash(repo, &entry.hash)?;
                let mut subfiles = subtree.list_files(repo, &full_path)?;
                files.append(&mut subfiles);
            }
        }
        
        Ok(files)
    }
}
