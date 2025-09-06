use crate::core::{Repository, Index};
use walkdir::WalkDir;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CodeAnalysis {
    pub total_files: usize,
    pub total_lines: usize,
    pub file_types: HashMap<String, usize>,
    pub largest_files: Vec<(String, usize)>,
    pub recent_changes: Vec<String>,
    pub complexity_score: f32,
    pub security_score: f32,
    pub maintainability_score: f32,
}

pub async fn analyze_codebase(repo: &Repository) -> String {
    let analysis = perform_comprehensive_analysis(repo).await;
    format_analysis_report(&analysis)
}

async fn perform_comprehensive_analysis(repo: &Repository) -> CodeAnalysis {
    let mut analysis = CodeAnalysis {
        total_files: 0,
        total_lines: 0,
        file_types: HashMap::new(),
        largest_files: Vec::new(),
        recent_changes: Vec::new(),
        complexity_score: 0.0,
        security_score: 100.0,
        maintainability_score: 100.0,
    };

    let mut file_sizes = Vec::new();
    let mut total_complexity = 0.0;
    let mut security_issues = 0;

    for entry in WalkDir::new(&repo.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.path().starts_with(&repo.git_dir))
        .filter(|e| !should_ignore_file(e.path()))
    {
        analysis.total_files += 1;

        if let Some(ext) = entry.path().extension() {
            if let Some(ext_str) = ext.to_str() {
                *analysis.file_types.entry(ext_str.to_string()).or_insert(0) += 1;
            }
        }

        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            let line_count = content.lines().count();
            analysis.total_lines += line_count;
            
            if let Some(path_str) = entry.path().to_str() {
                file_sizes.push((path_str.to_string(), line_count));
            }

            let file_complexity = calculate_file_complexity(&content);
            total_complexity += file_complexity;

            let file_security_issues = scan_security_patterns(&content);
            security_issues += file_security_issues;

            let maintainability_impact = calculate_maintainability(&content);
            analysis.maintainability_score -= maintainability_impact;
        }
    }

    file_sizes.sort_by(|a, b| b.1.cmp(&a.1));
    analysis.largest_files = file_sizes.into_iter().take(10).collect();

    if analysis.total_files > 0 {
        analysis.complexity_score = total_complexity / analysis.total_files as f32;
        analysis.security_score = (100.0 - (security_issues as f32 * 2.0)).max(0.0);
        analysis.maintainability_score = analysis.maintainability_score.max(0.0);
    }

    analysis.recent_changes = get_recent_changes(repo).await;

    analysis
}

fn should_ignore_file(path: &std::path::Path) -> bool {
    let ignore_patterns = [
        "target", "node_modules", ".git", "build", "dist", "__pycache__",
        ".pytest_cache", ".coverage", ".idea", ".vscode", "*.pyc", "*.log", 
        "*.tmp", ".DS_Store", "Thumbs.db", "*.min.js", "*.min.css"
    ];

    let path_str = path.to_string_lossy().to_lowercase();
    ignore_patterns.iter().any(|pattern| {
        if pattern.contains('*') {
            let pattern_clean = pattern.replace('*', "");
            path_str.contains(&pattern_clean)
        } else {
            path_str.contains(pattern)
        }
    })
}

fn calculate_file_complexity(content: &str) -> f32 {
    let mut complexity = 0.0;
    let lines = content.lines();
    let mut nesting_level = 0;
    
    for line in lines {
        let trimmed = line.trim();
        
        if trimmed.starts_with("fn ") || trimmed.starts_with("function ") || 
           trimmed.starts_with("def ") || trimmed.starts_with("class ") {
            complexity += 1.0;
        }
        
        if trimmed.contains("if ") || trimmed.contains("while ") || 
           trimmed.contains("for ") || trimmed.contains("match ") ||
           trimmed.contains("switch ") || trimmed.contains("case ") {
            complexity += 1.0 + (nesting_level as f32 * 0.1);
        }
        
        if trimmed.contains("&&") || trimmed.contains("||") || 
           trimmed.contains("and ") || trimmed.contains("or ") {
            complexity += 0.5;
        }
        
        if trimmed.contains("try ") || trimmed.contains("catch ") ||
           trimmed.contains("except ") || trimmed.contains("unwrap") {
            complexity += 0.5;
        }
        
        if trimmed.ends_with('{') || trimmed.ends_with(':') {
            nesting_level += 1;
        }
        if trimmed.starts_with('}') || (line.len() - line.trim_start().len() < nesting_level * 4 && nesting_level > 0) {
            nesting_level = nesting_level.saturating_sub(1);
        }
        
        if trimmed.len() > 120 {
            complexity += 0.2;
        }
        
        if line.len() - line.trim_start().len() > 40 {
            complexity += 0.1;
        }
    }
    
    complexity
}

fn scan_security_patterns(content: &str) -> usize {
    let security_patterns = [
        r#"(?i)(password|secret|key|token|api_key)\s*[:=]\s*['"][^'"]{3,}['"]"#,
        r"(?i)sql\s*\+|query\s*\+|\$\{.*\}.*select",
        r"eval\s*\(|exec\s*\(|system\s*\(|shell_exec",
        r"innerHTML\s*=|document\.write|\.html\(",
        r"-----BEGIN (RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----",
        r"AKIA[0-9A-Z]{16}",
        r"sk_live_[0-9a-zA-Z]{24}",
        r"(?i)unsafe\s+|\bunsafe\b",
        r"(?i)todo.*security|fixme.*security|hack.*security",
        r"(?i)md5\(|sha1\(",
    ];
    
    let mut issues = 0;
    for pattern in &security_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            issues += re.find_iter(content).count();
        }
    }
    
    issues
}

fn calculate_maintainability(content: &str) -> f32 {
    let mut maintainability_debt = 0.0;
    let lines: Vec<&str> = content.lines().collect();
    
    let mut comment_lines = 0;
    let mut code_lines = 0;
    
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("#") || 
           trimmed.starts_with("/*") || trimmed.starts_with("*") {
            comment_lines += 1;
        } else if !trimmed.is_empty() {
            code_lines += 1;
        }
        
        if trimmed.contains("TODO") || trimmed.contains("FIXME") || 
           trimmed.contains("HACK") || trimmed.contains("XXX") {
            maintainability_debt += 1.0;
        }
        
        if trimmed.len() > 120 {
            maintainability_debt += 0.1;
        }
    }
    
    if code_lines > 0 {
        let comment_ratio = comment_lines as f32 / code_lines as f32;
        if comment_ratio < 0.1 {
            maintainability_debt += 2.0;
        }
    }
    
    if lines.len() > 1000 {
        maintainability_debt += (lines.len() as f32 / 1000.0) * 0.5;
    }
    
    maintainability_debt
}

async fn get_recent_changes(repo: &Repository) -> Vec<String> {
    let index = Index::load(repo).unwrap_or_default();
    index.entries.keys().take(15).cloned().collect()
}

fn format_analysis_report(analysis: &CodeAnalysis) -> String {
    let mut report = String::new();
    
    report.push_str("=== Project Analysis Report ===\n");
    report.push_str(&format!("Total files: {}\n", analysis.total_files));
    report.push_str(&format!("Lines of code: {}\n", analysis.total_lines));
    report.push_str(&format!("Avg complexity: {:.1}\n", analysis.complexity_score));
    report.push_str(&format!("Security score: {:.0}/100\n", analysis.security_score));
    report.push_str(&format!("Maintainability: {:.0}/100\n", analysis.maintainability_score));
    
    if !analysis.file_types.is_empty() {
        report.push_str("\nFile distribution:\n");
        let mut sorted_types: Vec<_> = analysis.file_types.iter().collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1));
        
        for (ext, count) in sorted_types.iter().take(8) {
            let percentage = (**count as f32 / analysis.total_files as f32) * 100.0;
            report.push_str(&format!("   {} - {} files ({:.1}%)\n", ext, count, percentage));
        }
    }
    
    if !analysis.largest_files.is_empty() {
        report.push_str("\nLargest files:\n");
        for (file, lines) in analysis.largest_files.iter().take(5) {
            report.push_str(&format!("   {} - {} lines\n", file, lines));
        }
    }
    
    if !analysis.recent_changes.is_empty() {
        report.push_str("\nRecent changes:\n");
        for file in analysis.recent_changes.iter().take(8) {
            report.push_str(&format!("   {}\n", file));
        }
    }
    
    if analysis.complexity_score > 8.0 {
        report.push_str("\nHigh complexity detected - consider refactoring\n");
    }
    
    if analysis.security_score < 80.0 {
        report.push_str("\nSecurity issues found - review recommended\n");
    }
    
    if analysis.maintainability_score < 70.0 {
        report.push_str("\nMaintainability concerns - cleanup suggested\n");
    }
    
    report
}

pub async fn analyze_diff_complexity(diff: &str) -> f32 {
    let mut complexity = 0.0;
    let lines: Vec<&str> = diff.lines().collect();
    
    for line in &lines {
        if line.starts_with('+') || line.starts_with('-') {
            complexity += 1.0;
            
            let content = &line[1..];
            
            if content.contains("if ") || content.contains("for ") || 
               content.contains("while ") || content.contains("match ") {
                complexity += 2.0;
            }
            
            if content.contains("async") || content.contains("await") ||
               content.contains("Promise") || content.contains("Future") {
                complexity += 1.5;
            }
            
            if content.contains("unsafe") || content.contains("unwrap") ||
               content.contains("expect") {
                complexity += 2.5;
            }
            
            if content.contains("TODO") || content.contains("FIXME") ||
               content.contains("HACK") {
                complexity += 1.0;
            }
            
            if content.len() > 120 {
                complexity += 0.5;
            }
        }
    }
    
    if lines.is_empty() {
        0.0
    } else {
        complexity / lines.len() as f32
    }
}
