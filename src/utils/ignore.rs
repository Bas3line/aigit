use std::path::Path;
use regex::Regex;

pub struct GitIgnore {
    patterns: Vec<IgnorePattern>,
}

struct IgnorePattern {
    pattern: Regex,
    negated: bool,
    directory_only: bool,
}

impl GitIgnore {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        let gitignore_path = repo_path.as_ref().join(".gitignore");
        let patterns = if gitignore_path.exists() {
            Self::load_patterns(&gitignore_path)
        } else {
            Self::default_patterns()
        };
        
        Self { patterns }
    }

    fn load_patterns(gitignore_path: &Path) -> Vec<IgnorePattern> {
        let content = std::fs::read_to_string(gitignore_path).unwrap_or_default();
        let mut patterns = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some(pattern) = Self::parse_pattern(line) {
                patterns.push(pattern);
            }
        }
        
        patterns.extend(Self::default_patterns());
        patterns
    }

    fn default_patterns() -> Vec<IgnorePattern> {
        let defaults = [
            ".aigit/",
            "target/",
            "*.tmp",
            "*.log",
            ".env",
            ".DS_Store",
            "node_modules/",
            "*.swp",
            "*.swo",
            "__pycache__/",
            "*.pyc",
            ".pytest_cache/",
            "dist/",
            "build/",
            ".cache/",
            "*.orig",
            "*.rej",
        ];
        
        defaults
            .iter()
            .filter_map(|pattern| Self::parse_pattern(pattern))
            .collect()
    }

    fn parse_pattern(pattern_str: &str) -> Option<IgnorePattern> {
        let mut pattern = pattern_str;
        let mut negated = false;
        let mut directory_only = false;
        
        if pattern.starts_with('!') {
            negated = true;
            pattern = &pattern[1..];
        }
        
        if pattern.ends_with('/') {
            directory_only = true;
            pattern = &pattern[..pattern.len()-1];
        }
        
        let regex_pattern = Self::glob_to_regex(pattern);
        
        Regex::new(&regex_pattern).ok().map(|regex| IgnorePattern {
            pattern: regex,
            negated,
            directory_only,
        })
    }

    fn glob_to_regex(glob: &str) -> String {
        let mut regex = String::new();
        regex.push_str("^");
        
        let chars: Vec<char> = glob.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            match chars[i] {
                '*' => {
                    if i + 1 < chars.len() && chars[i + 1] == '*' {
                        regex.push_str(".*");
                        i += 2;
                    } else {
                        regex.push_str("[^/]*");
                        i += 1;
                    }
                },
                '?' => {
                    regex.push_str("[^/]");
                    i += 1;
                },
                '[' => {
                    regex.push('[');
                    i += 1;
                    while i < chars.len() && chars[i] != ']' {
                        regex.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        regex.push(']');
                        i += 1;
                    }
                },
                c if "(){}^$.|\\+".contains(c) => {
                    regex.push('\\');
                    regex.push(c);
                    i += 1;
                },
                c => {
                    regex.push(c);
                    i += 1;
                }
            }
        }
        
        regex.push('$');
        regex
    }

    pub fn is_ignored<P: AsRef<Path>>(&self, path: P) -> bool {
        let path_str = path.as_ref().to_string_lossy();
        let is_dir = path.as_ref().is_dir();
        
        let mut ignored = false;
        
        for pattern in &self.patterns {
            if pattern.directory_only && !is_dir {
                continue;
            }
            
            if pattern.pattern.is_match(&path_str) {
                ignored = !pattern.negated;
            }
        }
        
        ignored
    }
}
