# AIGIT Documentation

AIGIT is a modern, AI-powered version control system built in Rust. It provides traditional version control functionality enhanced with artificial intelligence features for better code management.

## Table of Contents
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Commands](#commands)
- [Configuration](#configuration)
- [Advanced Features](#advanced-features)
- [Troubleshooting](#troubleshooting)

## Installation

### Building from Source

1. Ensure you have Rust installed (https://rustup.rs/)
2. Clone the repository
3. Build the project:
   ```bash
   cargo build --release
   ```
4. The binary will be available at `target/release/aigit`

### Adding to PATH (Optional)
Copy the binary to a directory in your PATH or add the target/release directory to your PATH.

## Getting Started

### Initialize a Repository

```bash
aigit init
```

This creates a new `.aigit` directory in your current folder, similar to `.git` but for the aigit system.

### Configure User Information

Set your name and email for commits:

```bash
aigit config user --name "Your Name" --email "your.email@example.com"
```

### Basic Workflow

1. **Add files to staging area:**
   ```bash
   aigit add file1.txt file2.txt
   # Or add all files
   aigit add --all
   ```

2. **Commit changes:**
   ```bash
   aigit commit --message "Your commit message"
   # Or let AI generate a commit message
   aigit commit
   ```

3. **Check repository status:**
   ```bash
   aigit status
   ```

4. **View commit history:**
   ```bash
   aigit log
   ```

## Commands

### Core Commands

#### `aigit init`
Initialize a new aigit repository in the current directory.

Options:
- `--bare`: Create a bare repository

#### `aigit add <files...>`
Add files to the staging area.

Options:
- `--all` or `-a`: Add all modified files

Example:
```bash
aigit add src/main.rs src/lib.rs
aigit add --all
```

#### `aigit commit`
Create a new commit with staged changes.

Options:
- `--message <msg>` or `-m <msg>`: Specify commit message
- `--amend`: Amend the previous commit
- `--ai-review`: Enable AI code review before committing
- `--signoff` or `-s`: Add a signed-off-by line

Examples:
```bash
aigit commit --message "Fix bug in authentication"
aigit commit --ai-review
aigit commit --amend
```

#### `aigit status`
Show the working tree status.

Options:
- `--porcelain` or `-p`: Give output in porcelain format

#### `aigit log`
Show commit history.

Options:
- `--oneline` or `-o`: Show each commit on one line
- `--graph` or `-g`: Show a text-based graphical representation
- `--ai-summary`: Generate AI summary of changes

#### `aigit push <branch>`
Synchronize a branch locally (for collaboration readiness).

Example:
```bash
aigit push main
aigit push feature-branch
```

### Branch Management

#### `aigit branch [name]`
List branches or create a new branch.

Options:
- `--delete <branch>` or `-d <branch>`: Delete a branch
- `--ai-suggest`: Get AI suggestions for branch names

Examples:
```bash
aigit branch                    # List all branches
aigit branch new-feature        # Create new branch
aigit branch --delete old-feature
```

#### `aigit checkout <target>`
Switch branches or restore working tree files.

Options:
- `--create` or `-c`: Create and switch to new branch

Examples:
```bash
aigit checkout main
aigit checkout --create new-feature
```

### Comparison and Analysis

#### `aigit diff`
Show changes between commits, commit and working tree, etc.

Options:
- `--cached`: Show changes between index and last commit
- `--ai-explain`: Get AI explanation of changes

#### `aigit merge <branch>`
Merge changes from another branch.

Options:
- `--ai-resolve`: Use AI to help resolve conflicts

### AI-Enhanced Features

#### `aigit review`
Perform AI-powered code review.

Options:
- `--full`: Perform comprehensive review

#### `aigit suggest`
Get AI suggestions for various operations.

Subcommands:
- `commit`: Suggest commit messages
- `branch`: Suggest branch names
- `refactor`: Suggest code refactoring
- `tests`: Suggest test improvements
- `cleanup`: Suggest code cleanup

Examples:
```bash
aigit suggest commit
aigit suggest refactor
aigit suggest tests
```

### Configuration

#### `aigit config`
Get and set repository or global options.

Subcommands:
- `set <key> <value>`: Set configuration value
- `get <key>`: Get configuration value
- `list`: List all configuration
- `user --name <name> --email <email>`: Set user information

Examples:
```bash
aigit config set core.editor vim
aigit config get user.name
aigit config list
aigit config user --name "John Doe" --email "john@example.com"
```

## Configuration

AIGIT stores configuration in two places:
- Global config: `~/.aigit/config`
- Repository config: `.aigit/config`

### Common Configuration Options

```ini
[user]
name = Your Name
email = your.email@example.com

[core]
editor = vim
autocrlf = false

[ai]
model = gemini
api_key = your-api-key
```

### Setting Up AI Features

To use AI features, you need to configure your AI service:

```bash
aigit config set ai.model gemini
aigit config set ai.api_key your-gemini-api-key
```

## Advanced Features

### Security Features

AIGIT includes built-in security features:
- Automatic scanning for sensitive files
- Pre-commit security checks
- File size limits (100MB per file, 1GB per commit)
- Suspicious content detection
- Audit logging

### Audit Logging

All operations are logged to `.aigit/logs/audit.log` for security and compliance purposes.

### File Ignore Patterns

Create a `.aigitignore` file to specify files that should be ignored:

```
*.log
*.tmp
node_modules/
target/
.env
```

## Troubleshooting

### Common Issues

#### "File was modified after staging"
This occurs when a file is changed after being added to the staging area. Re-add the file:
```bash
aigit add filename
aigit commit --message "Your message"
```

#### "Branch has no commits"
When pushing to a new branch, ensure you're on the correct branch:
```bash
aigit checkout target-branch
aigit push target-branch
```

#### "Not in a repository"
Ensure you're in a directory with an `.aigit` folder:
```bash
aigit init  # If starting fresh
```

### Performance Tips

1. Use specific file paths instead of `--all` for large repositories
2. Regularly clean up old branches
3. Keep individual commits focused and small
4. Use `.aigitignore` to exclude unnecessary files

### Getting Help

Use the `--help` flag with any command to get detailed information:

```bash
aigit --help
aigit commit --help
aigit branch --help
```

## File Structure

An AIGIT repository has the following structure:

```
.aigit/
├── config              # Repository configuration
├── HEAD               # Current branch reference
├── index              # Staging area
├── objects/           # Object database
│   ├── xx/           # First two characters of hash
│   └── ...
├── refs/              # References
│   ├── heads/         # Local branches
│   └── tags/          # Tags
└── logs/              # Operation logs
    └── audit.log      # Audit trail
```

This documentation covers the main features of AIGIT. For more advanced usage and development information, please refer to the source code and comments.
