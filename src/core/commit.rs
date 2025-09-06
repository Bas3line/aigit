use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone)]
pub struct Commit {
    pub tree: String,
    pub parent: Option<String>,
    pub parents: Vec<String>,
    pub author: Author,
    pub committer: Author,
    pub message: String,
    pub signature: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Author {
    pub name: String,
    pub email: String,
    pub timestamp: DateTime<Utc>,
}

impl Commit {
    pub fn new(
        tree: String, 
        parent: Option<String>, 
        author_name: String, 
        author_email: String, 
        message: String
    ) -> Self {
        let timestamp = Utc::now();
        let author = Author {
            name: author_name.clone(),
            email: author_email.clone(),
            timestamp,
        };

        let parents = if let Some(ref p) = parent {
            vec![p.clone()]
        } else {
            vec![]
        };

        Self {
            tree,
            parent,
            parents,
            author: author.clone(),
            committer: author,
            message,
            signature: None,
            timestamp,
        }
    }

    pub fn new_secure(
        tree: String,
        parent: Option<String>,
        author_name: String,
        author_email: String,
        message: String,
        signature: String,
    ) -> Self {
        let mut commit = Self::new(tree, parent, author_name, author_email, message);
        commit.signature = Some(signature);
        commit
    }

    pub fn new_merge(
        tree: String,
        parents: Vec<String>,
        author_name: String,
        author_email: String,
        message: String,
        signature: String,
    ) -> Self {
        let timestamp = Utc::now();
        let author = Author {
            name: author_name.clone(),
            email: author_email.clone(),
            timestamp,
        };

        let parent = parents.get(0).cloned();

        Self {
            tree,
            parent,
            parents,
            author: author.clone(),
            committer: author,
            message,
            signature: Some(signature),
            timestamp,
        }
    }

    pub fn short_hash(&self, hash: &str) -> String {
        hash.chars().take(8).collect()
    }

    pub fn short_message(&self) -> String {
        self.message.lines().next().unwrap_or("").to_string()
    }

    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    pub fn get_commit_size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.message.trim().is_empty() {
            return Err("Commit message cannot be empty".to_string());
        }

        if self.tree.is_empty() {
            return Err("Tree hash cannot be empty".to_string());
        }

        if self.tree.len() < 8 || !self.tree.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid tree hash format".to_string());
        }

        if self.author.name.trim().is_empty() {
            return Err("Author name cannot be empty".to_string());
        }

        if self.author.email.trim().is_empty() || !self.author.email.contains('@') {
            return Err("Invalid author email".to_string());
        }

        for parent in &self.parents {
            if parent.len() < 8 || !parent.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("Invalid parent hash format: {}", parent));
            }
        }

        if self.message.len() > 100000 {
            return Err("Commit message too long (max 100,000 characters)".to_string());
        }

        Ok(())
    }

    pub fn get_parents_string(&self) -> String {
        self.parents.join(" ")
    }

    pub fn format_for_display(&self, hash: &str) -> String {
        let short_hash = self.short_hash(hash);
        let short_msg = self.short_message();
        let author_date = self.author.timestamp.format("%Y-%m-%d %H:%M:%S");
        
        format!("{} {} - {} ({})", 
                short_hash, 
                short_msg, 
                self.author.name, 
                author_date)
    }
}

impl Author {
    pub fn new(name: String, email: String) -> Self {
        Self {
            name,
            email,
            timestamp: Utc::now(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Author name cannot be empty".to_string());
        }

        if self.email.trim().is_empty() {
            return Err("Author email cannot be empty".to_string());
        }

        if !self.email.contains('@') || !self.email.contains('.') {
            return Err("Invalid email format".to_string());
        }

        if self.name.len() > 255 {
            return Err("Author name too long".to_string());
        }

        if self.email.len() > 255 {
            return Err("Author email too long".to_string());
        }

        Ok(())
    }

    pub fn format_signature(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}
    