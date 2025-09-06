# Contributing to AIGIT

Thank you for your interest in contributing to AIGIT! We welcome contributions from developers of all skill levels. This document provides guidelines and information to help you contribute effectively.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Contributing Guidelines](#contributing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Code Style and Standards](#code-style-and-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Reporting Issues](#reporting-issues)
- [Feature Requests](#feature-requests)
- [Security Issues](#security-issues)

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please treat all contributors with respect and create a welcoming environment for everyone.

### Our Standards

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.70+** (2021 edition or later)
- **Git** for version control
- **Cargo** (comes with Rust)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/Bas3line/aigit.git
   cd aigit
   ```
3. Add the original repository as upstream:
   ```bash
   git remote add upstream https://github.com/Bas3line/aigit.git
   ```

## Development Setup

### Initial Setup

1. Install dependencies and build the project:
   ```bash
   cargo build
   ```

2. Run tests to ensure everything works:
   ```bash
   cargo test
   ```

3. Install development tools:
   ```bash
   rustup component add rustfmt clippy
   ```

### Running AIGIT Locally

```bash
# Run with cargo
cargo run -- --help

# Or build and run the binary
cargo build --release
./target/release/aigit --help
```

### Development Commands

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Generate documentation
cargo doc --open
```

## Contributing Guidelines

### Types of Contributions

We welcome various types of contributions:

- **Bug fixes**: Fix existing issues or problems
- **Features**: Implement new functionality
- **Documentation**: Improve or add documentation
- **Tests**: Add or improve test coverage
- **Performance**: Optimize existing code
- **Security**: Enhance security features
- **AI Integration**: Improve AI capabilities

### Before You Start

1. **Check existing issues**: Look for existing issues or discussions about your proposed change
2. **Create an issue**: For significant changes, create an issue to discuss your approach
3. **Get feedback**: Engage with maintainers and community members
4. **Start small**: Begin with smaller contributions to understand the codebase

### Branch Naming

Use descriptive branch names with the following prefixes:

- `feature/` - New features
- `bugfix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test improvements
- `security/` - Security enhancements

Examples:
- `feature/ai-commit-suggestions`
- `bugfix/branch-deletion-crash`
- `docs/installation-guide`

## Pull Request Process

### Creating a Pull Request

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**: Follow the code style guidelines

3. **Test your changes**: Ensure all tests pass

4. **Commit your changes**: Use meaningful commit messages
   ```bash
   git commit -m "Add AI-powered commit message generation"
   ```

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a pull request** on GitHub

### Pull Request Requirements

Your pull request must:

- **Pass all tests**: Ensure `cargo test` succeeds
- **Follow code style**: Run `cargo fmt` and `cargo clippy`
- **Include tests**: Add tests for new functionality
- **Update documentation**: Update relevant documentation
- **Have a clear description**: Explain what changes you made and why

### Pull Request Template

When creating a pull request, please include:

```markdown
## Description
Brief description of the changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Security enhancement

## Testing
- [ ] Tests pass locally
- [ ] Added new tests for new functionality
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No breaking changes (or properly documented)
```

## Code Style and Standards

### Rust Style Guidelines

We follow standard Rust conventions:

- Use `rustfmt` for consistent formatting
- Follow Rust naming conventions (snake_case, PascalCase, etc.)
- Use meaningful variable and function names
- Add documentation comments for public APIs
- Prefer explicit error handling over panics

### Code Organization

- **Modules**: Keep modules focused and cohesive
- **Functions**: Keep functions small and focused
- **Error Handling**: Use `Result<T, E>` for recoverable errors
- **Documentation**: Document public APIs with examples

### Example Code Style

```rust
/// Validates a branch name according to AIGIT rules
/// 
/// # Arguments
/// * `name` - The branch name to validate
/// 
/// # Returns
/// * `Ok(())` if valid
/// * `Err(String)` with validation error message
/// 
/// # Examples
/// ```
/// assert!(validate_branch_name("feature/new-ui").is_ok());
/// assert!(validate_branch_name("").is_err());
/// ```
pub fn validate_branch_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Branch name cannot be empty".to_string());
    }
    
    // Additional validation logic...
    Ok(())
}
```

## Testing

### Test Requirements

- All new code must include appropriate tests
- Tests should cover both success and error cases
- Use descriptive test names
- Group related tests in modules

### Test Types

- **Unit Tests**: Test individual functions and modules
- **Integration Tests**: Test component interactions
- **Documentation Tests**: Ensure code examples work

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests for specific module
cargo test module_name
```

### Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_branch_name_success() {
        assert!(validate_branch_name("feature/test").is_ok());
    }
    
    #[test]
    fn test_validate_branch_name_empty() {
        assert!(validate_branch_name("").is_err());
    }
}
```

## Documentation

### Documentation Standards

- Use clear, concise language
- Include code examples where appropriate
- Document all public APIs
- Update README.md when adding major features
- Use proper Rust documentation format

### Documentation Commands

```bash
# Generate and open documentation
cargo doc --open

# Check documentation
cargo doc --no-deps
```

## Reporting Issues

### Before Reporting

1. **Search existing issues**: Check if the issue already exists
2. **Use latest version**: Ensure you're using the latest version
3. **Minimal reproduction**: Create a minimal example that reproduces the issue

### Issue Template

When reporting bugs, please include:

- **AIGIT version**: Output of `aigit --version`
- **Operating system**: OS and version
- **Rust version**: Output of `rustc --version`
- **Steps to reproduce**: Clear steps to reproduce the issue
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Additional context**: Any other relevant information

## Feature Requests

### Proposing Features

1. **Check existing requests**: Look for similar feature requests
2. **Create detailed issue**: Describe the feature and its benefits
3. **Discuss implementation**: Engage with maintainers about approach
4. **Consider alternatives**: Think about alternative solutions

### Feature Request Template

- **Feature description**: Clear description of the proposed feature
- **Use case**: Why this feature would be useful
- **Proposed implementation**: How you think it should work
- **Alternatives**: Other ways to achieve the same goal

## Security Issues

### Reporting Security Vulnerabilities

Do **NOT** open public issues for security vulnerabilities. Instead:

1. **Email directly**: Send details to wilfred.shubham@gmail.com
2. **Include details**: Provide as much information as possible
3. **Wait for response**: Allow time for investigation and fix
4. **Coordinated disclosure**: Work with maintainers on disclosure timing

## Getting Help

If you need help with contributing:

- **GitHub Discussions**: Use GitHub Discussions for questions
- **Issues**: Create an issue with the "question" label
- **Documentation**: Check existing documentation first

## Recognition

We value all contributions and will:

- Add contributors to the AUTHORS file
- Mention significant contributions in release notes
- Provide feedback and guidance for improvement

## Development Status Notice

Please note that AIGIT is currently in active development. Some features may not work as intended, and breaking changes may occur. Contributors should be prepared for:

- API changes between versions
- Incomplete features
- Experimental functionality
- Regular refactoring

Thank you for contributing to AIGIT! Your efforts help make this project better for everyone.
