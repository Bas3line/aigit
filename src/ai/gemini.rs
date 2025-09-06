use reqwest::Client;
use serde_json::json;
use std::env;
use std::time::Duration;

pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let api_key = env::var("GEMINI_API_KEY")
            .or_else(|_| {
                if std::path::Path::new(".env").exists() {
                    let env_content = std::fs::read_to_string(".env")
                        .expect("Failed to read .env file");
                    
                    for line in env_content.lines() {
                        if line.starts_with("GEMINI_API_KEY=") {
                            return Ok(line.strip_prefix("GEMINI_API_KEY=").unwrap_or("").to_string());
                        }
                    }
                }
                Err(env::VarError::NotPresent)
            })
            .expect("GEMINI_API_KEY must be set in environment or .env file");

        Self {
            client,
            api_key,
        }
    }

    pub async fn generate_commit_message(&self, diff: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Generate a concise git commit message for these changes. \
            Use conventional commit format (feat:, fix:, docs:, style:, refactor:, test:, chore:). \
            Keep it under 60 characters and focus on the main change:\n\n{}",
            diff.chars().take(2500).collect::<String>()
        );

        let response = self.generate_text(&prompt).await?;
        Ok(response.lines().next().unwrap_or("chore: update files").trim().to_string())
    }

    pub async fn review_code(&self, diff: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Provide a thorough code review for these changes. Focus on:\n\
            - Potential bugs and logical errors\n\
            - Code quality and best practices\n\
            - Security vulnerabilities\n\
            - Performance implications\n\
            - Maintainability concerns\n\
            Be constructive and specific with suggestions.\n\n\
            Changes:\n{}",
            diff.chars().take(4000).collect::<String>()
        );

        self.generate_text(&prompt).await
    }

    pub async fn comprehensive_review(&self, diff: &str, detailed: bool) -> Result<String, Box<dyn std::error::Error>> {
        let analysis_depth = if detailed { 
            "comprehensive and detailed" 
        } else { 
            "focused and concise" 
        };

        let prompt = format!(
            "Provide a {} code review for these changes:\n\n\
            **Code Quality Analysis:**\n\
            - Adherence to best practices and coding standards\n\
            - Code structure and organization\n\
            - Readability and maintainability\n\n\
            **Bug Detection:**\n\
            - Potential runtime errors\n\
            - Logic flaws and edge cases\n\
            - Type safety issues\n\n\
            **Security Assessment:**\n\
            - Vulnerability patterns\n\
            - Input validation\n\
            - Data exposure risks\n\n\
            **Performance Review:**\n\
            - Algorithmic efficiency\n\
            - Resource usage\n\
            - Scalability concerns\n\n\
            **Architecture & Design:**\n\
            - Design patterns usage\n\
            - Separation of concerns\n\
            - Testability\n\n\
            Changes to review:\n{}",
            analysis_depth,
            diff.chars().take(5000).collect::<String>()
        );

        self.generate_text(&prompt).await
    }

    pub async fn suggest_improvements(&self, diff: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Based on these code changes, provide specific improvement suggestions:\n\n\
            **Immediate Improvements:**\n\
            - Code optimizations\n\
            - Bug fixes\n\
            - Style improvements\n\n\
            **Enhancement Opportunities:**\n\
            - Performance optimizations\n\
            - Feature additions\n\
            - Error handling improvements\n\n\
            **Long-term Considerations:**\n\
            - Refactoring opportunities\n\
            - Architecture improvements\n\
            - Technical debt reduction\n\n\
            Code changes:\n{}",
            diff.chars().take(4000).collect::<String>()
        );

        self.generate_text(&prompt).await
    }

    pub async fn explain_diff(&self, diff: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Explain what these code changes accomplish in clear, non-technical terms. \
            Focus on:\n\
            - What functionality is being added/modified/removed\n\
            - Why these changes might be necessary\n\
            - The impact on the overall system\n\
            - Any notable patterns or approaches used\n\n\
            Changes:\n{}",
            diff.chars().take(3000).collect::<String>()
        );

        self.generate_text(&prompt).await
    }

    pub async fn suggest_next_commit(&self, context: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Based on this project analysis, suggest what should be worked on next. \
            Consider:\n\
            - High-priority bugs or security issues\n\
            - Important missing features\n\
            - Code quality improvements\n\
            - Technical debt reduction\n\
            - Performance optimizations\n\
            Provide actionable recommendations with reasoning.\n\n\
            Project context:\n{}",
            context
        );

        self.generate_text(&prompt).await
    }

    pub async fn suggest_branch_name(&self, context: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Suggest 5 good branch names for upcoming development work based on this project. \
            Use conventional naming:\n\
            - feature/ for new features\n\
            - bugfix/ or fix/ for bug fixes\n\
            - hotfix/ for critical fixes\n\
            - refactor/ for code improvements\n\
            - chore/ for maintenance tasks\n\
            - docs/ for documentation\n\
            - test/ for testing improvements\n\
            Make them descriptive but concise.\n\n\
            Project context:\n{}",
            context
        );

        let response = self.generate_text(&prompt).await?;
        let suggestions: Vec<String> = response
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with(char::is_numeric) || 
                   trimmed.starts_with("- ") || 
                   trimmed.starts_with("* ") ||
                   trimmed.starts_with("• ") {
                    Some(extract_branch_name(trimmed))
                } else if !trimmed.is_empty() && 
                         (trimmed.contains('/') || !trimmed.contains(' ')) &&
                         trimmed.len() < 50 {
                    Some(trimmed.to_string())
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        if suggestions.is_empty() {
            Ok(vec![
                "feature/new-functionality".to_string(),
                "bugfix/critical-issue".to_string(),
                "refactor/code-cleanup".to_string(),
                "chore/dependency-update".to_string(),
                "docs/api-documentation".to_string(),
            ])
        } else {
            Ok(suggestions)
        }
    }

    pub async fn suggest_refactoring(&self, context: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Analyze this codebase and suggest refactoring opportunities:\n\n\
            **Code Analysis:**\n\
            - Identify code smells and anti-patterns\n\
            - Find duplicated code\n\
            - Locate overly complex functions\n\n\
            **Refactoring Suggestions:**\n\
            - Extract methods/functions\n\
            - Simplify conditional logic\n\
            - Improve naming conventions\n\
            - Reduce coupling\n\n\
            **Impact Assessment:**\n\
            - Priority level (high/medium/low)\n\
            - Effort estimation\n\
            - Benefits and risks\n\n\
            Codebase context:\n{}",
            context
        );

        self.generate_text(&prompt).await
    }

    pub async fn suggest_tests(&self, context: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Analyze this codebase for testing opportunities:\n\n\
            **Test Coverage Analysis:**\n\
            - Identify untested code paths\n\
            - Find critical functions without tests\n\
            - Locate edge cases that need testing\n\n\
            **Test Recommendations:**\n\
            - Unit tests for core functionality\n\
            - Integration tests for component interaction\n\
            - Error handling and edge case tests\n\
            - Performance and load tests\n\n\
            **Priority Suggestions:**\n\
            - High-risk areas that need immediate testing\n\
            - Complex logic that benefits from test coverage\n\
            - Public APIs that require comprehensive testing\n\n\
            Codebase analysis:\n{}",
            context
        );

        self.generate_text(&prompt).await
    }

    pub async fn analyze_merge(&self, context: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Analyze this merge operation and provide insights:\n\n\
            **Merge Strategy Analysis:**\n\
            - Compatibility assessment\n\
            - Potential conflict areas\n\
            - Risk evaluation\n\n\
            **Conflict Prevention:**\n\
            - Identify likely merge conflicts\n\
            - Suggest resolution strategies\n\
            - Recommend pre-merge actions\n\n\
            **Recommendations:**\n\
            - Best merge approach\n\
            - Testing requirements\n\
            - Post-merge verification steps\n\n\
            Merge context:\n{}",
            context
        );

        self.generate_text(&prompt).await
    }

    pub async fn resolve_conflict(&self, conflict_content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Help resolve this merge conflict by analyzing both sides and suggesting the best resolution:\n\n\
            **Conflict Analysis:**\n\
            - Understand what each side is trying to achieve\n\
            - Identify the root cause of the conflict\n\
            - Assess the importance of each change\n\n\
            **Resolution Strategy:**\n\
            - Suggest which version to keep or how to merge both\n\
            - Explain the reasoning behind the recommendation\n\
            - Highlight any additional considerations\n\n\
            Conflict content:\n{}",
            conflict_content
        );

        self.generate_text(&prompt).await
    }

    pub async fn generate_text(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let payload = json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": 0.7,
                "topK": 40,
                "topP": 0.95,
                "maxOutputTokens": 4096,
                "stopSequences": []
            },
            "safetySettings": [
                {
                    "category": "HARM_CATEGORY_HARASSMENT",
                    "threshold": "BLOCK_MEDIUM_AND_ABOVE"
                },
                {
                    "category": "HARM_CATEGORY_HATE_SPEECH", 
                    "threshold": "BLOCK_MEDIUM_AND_ABOVE"
                }
            ]
        });

        let response = self
            .client
            .post(&format!(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
                self.api_key
            ))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("Gemini API error: {} - {}", status, error_text).into());
        }

        let json: serde_json::Value = response.json().await?;
        
        if let Some(error) = json.get("error") {
            return Err(format!("Gemini API error: {}", error).into());
        }
        
        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "No response from Gemini API".into())
    }
}

fn extract_branch_name(line: &str) -> String {
    let cleaned = line
        .trim_start_matches(char::is_numeric)
        .trim_start_matches(". ")
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim_start_matches("• ")
        .trim();
    
    if let Some(space_pos) = cleaned.find(' ') {
        cleaned[..space_pos].to_string()
    } else {
        cleaned.to_string()
    }
}
