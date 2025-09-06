use std::fs;
use std::path::Path;
use aigit::core::{Repository, Index, Config};
use tokio;

#[tokio::test]
async fn test_init_repository() {
    let test_dir = "test_repos/init_test";
    cleanup_test_dir(test_dir);
    
    fs::create_dir_all(test_dir).unwrap();
    std::env::set_current_dir(test_dir).unwrap();
    
    let result = Repository::init(".", false);
    assert!(result.is_ok());
    
    let repo = result.unwrap();
    assert!(repo.git_dir.join("HEAD").exists());
    assert!(repo.git_dir.join("objects").exists());
    assert!(repo.git_dir.join("refs/heads").exists());
    assert!(repo.git_dir.join("security").exists());
    
    std::env::set_current_dir("../..").unwrap();
    cleanup_test_dir(test_dir);
}

#[tokio::test]
async fn test_config_operations() {
    let test_dir = "test_repos/config_test";
    cleanup_test_dir(test_dir);
    
    fs::create_dir_all(test_dir).unwrap();
    std::env::set_current_dir(test_dir).unwrap();
    
    let repo = Repository::init(".", false).unwrap();
    let mut config = Config::new();
    
    config.set("user.name", "Test User");
    config.set("user.email", "test@example.com");
    
    assert_eq!(config.get("user.name"), Some(&"Test User".to_string()));
    assert_eq!(config.get("user.email"), Some(&"test@example.com".to_string()));
    
    config.save_repo(&repo).unwrap();
    let loaded_config = Config::load_repo(&repo).unwrap();
    
    assert_eq!(loaded_config.get("user.name"), Some(&"Test User".to_string()));
    
    std::env::set_current_dir("../..").unwrap();
    cleanup_test_dir(test_dir);
}

#[tokio::test]
async fn test_index_operations() {
    let test_dir = "test_repos/index_test";
    cleanup_test_dir(test_dir);
    
    fs::create_dir_all(test_dir).unwrap();
    std::env::set_current_dir(test_dir).unwrap();
    
    let repo = Repository::init(".", false).unwrap();
    let mut index = Index::new();
    
    fs::write("test.txt", "Hello, World!").unwrap();
    
    index.add_entry("test.txt".to_string(), "dummy_hash".to_string(), "100644".to_string());
    assert!(!index.is_empty());
    assert!(index.entries.contains_key("test.txt"));
    
    index.save(&repo).unwrap();
    let loaded_index = Index::load(&repo).unwrap();
    
    assert!(loaded_index.entries.contains_key("test.txt"));
    
    std::env::set_current_dir("../..").unwrap();
    cleanup_test_dir(test_dir);
}

#[tokio::test]
async fn test_security_features() {
    let test_dir = "test_repos/security_test";
    cleanup_test_dir(test_dir);
    
    fs::create_dir_all(test_dir).unwrap();
    std::env::set_current_dir(test_dir).unwrap();
    
    let repo = Repository::init(".", false).unwrap();
    
    assert!(repo.security_dir().exists());
    assert!(repo.logs_dir().exists());
    
    let security_config = repo.get_security_config();
    assert!(security_config.is_some());
    
    repo.verify_integrity().unwrap();
    
    std::env::set_current_dir("../..").unwrap();
    cleanup_test_dir(test_dir);
}

fn cleanup_test_dir(dir: &str) {
    if Path::new(dir).exists() {
        fs::remove_dir_all(dir).ok();
    }
}
