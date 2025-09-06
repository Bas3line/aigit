use crate::core::{Repository, Index, Object};
use similar::{ChangeTag, TextDiff};

pub async fn generate_diff(repo: &Repository, index: &Index, staged: bool) -> Result<String, Box<dyn std::error::Error>> {
    let diff_output = if staged {
        generate_staged_diff(repo, index).await
    } else {
        generate_working_diff(repo, index).await
    };
    
    Ok(diff_output)
}

pub async fn get_staged_diff(repo: &Repository, index: &Index) -> String {
    generate_staged_diff(repo, index).await
}

async fn generate_staged_diff(repo: &Repository, index: &Index) -> String {
    let mut diff_output = String::new();
    
    for (file_path, _hash) in &index.entries {
        if let Ok(current_content) = std::fs::read_to_string(file_path) {
            let old_content = get_file_from_last_commit(repo, file_path).unwrap_or_default();
            
            if old_content != current_content {
                let old_lines: Vec<&str> = old_content.lines().collect();
                let current_lines: Vec<&str> = current_content.lines().collect();
                let diff = TextDiff::from_slices(&old_lines, &current_lines);
                diff_output.push_str(&format_diff_header(file_path, "staged"));
                diff_output.push_str(&format_diff_content(&diff));
            }
        }
    }
    
    diff_output
}

async fn generate_working_diff(repo: &Repository, index: &Index) -> String {
    let mut diff_output = String::new();
    
    for (file_path, staged_hash) in &index.entries {
        if let Ok(current_content) = std::fs::read_to_string(file_path) {
            let current_hash = crate::core::object::hash_content(current_content.as_bytes());
            
            if &current_hash != staged_hash {
                let staged_content = get_blob_content(repo, staged_hash).unwrap_or_default();
                let staged_lines: Vec<&str> = staged_content.lines().collect();
                let current_lines: Vec<&str> = current_content.lines().collect();
                let diff = TextDiff::from_slices(&staged_lines, &current_lines);
                
                diff_output.push_str(&format_diff_header(file_path, "working"));
                diff_output.push_str(&format_diff_content(&diff));
            }
        }
    }
    
    diff_output
}

fn format_diff_header(file_path: &str, diff_type: &str) -> String {
    format!("diff --aigit a/{} b/{} ({})\n--- a/{}\n+++ b/{}\n", 
            file_path, file_path, diff_type, file_path, file_path)
}

fn format_diff_content(diff: &TextDiff<str>) -> String {
    let mut output = String::new();
    
    for group in diff.grouped_ops(3) {
        let mut group_output = String::new();
        let mut first_old_line = 0;
        let mut first_new_line = 0;
        let mut old_count = 0;
        let mut new_count = 0;
        
        for op in &group {
            if first_old_line == 0 {
                first_old_line = op.old_range().start + 1;
                first_new_line = op.new_range().start + 1;
            }
            old_count += op.old_range().len();
            new_count += op.new_range().len();
            
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                group_output.push_str(&format!("{}{}", sign, change));
            }
        }
        
        output.push_str(&format!("@@ -{},{} +{},{} @@\n", 
                                first_old_line, old_count, 
                                first_new_line, new_count));
        output.push_str(&group_output);
    }
    
    output
}

fn get_file_from_last_commit(_repo: &Repository, _file_path: &str) -> Option<String> {
    None
}

fn get_blob_content(repo: &Repository, hash: &str) -> Option<String> {
    Object::read(repo, hash)
        .ok()
        .and_then(|content| String::from_utf8(content).ok())
}

pub async fn calculate_diff_stats(diff: &str) -> (usize, usize, usize) {
    let mut additions = 0;
    let mut deletions = 0;
    let mut modifications = 0;
    
    for line in diff.lines() {
        match line.chars().next() {
            Some('+') if !line.starts_with("+++") => additions += 1,
            Some('-') if !line.starts_with("---") => deletions += 1,
            Some(' ') => modifications += 1,
            _ => {}
        }
    }
    
    (additions, deletions, modifications)
}
