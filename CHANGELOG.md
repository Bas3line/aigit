# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2025-09-06

### Added
- New `push` command to synchronize branches locally
- Branch validation for push operations
- Commit count tracking for branches
- Audit logging for push operations
- Support for pushing to branches with no prior commits

### Fixed
- Fixed commit command failing with "File was modified after staging" error
- Added file existence check in security pre-commit validation
- Improved error handling for missing files during commit
- Fixed branch synchronization logic

### Changed
- Push command now works with aigit's local branch system instead of requiring git remotes
- Push command validates current branch matches target branch
- Improved error messages for push operations
- Enhanced branch validation to allow empty branches

### Technical Details
- Modified `security_pre_commit_checks` function to handle missing files gracefully
- Implemented `validate_branch_exists`, `get_current_branch`, and `get_branch_commit_count` functions
- Added comprehensive error handling for branch operations
- Integrated push command into main CLI interface

## [0.1.0] - Initial Release

### Added
- Core aigit version control system
- Basic repository initialization
- File staging and commit functionality
- Branch management
- AI-powered features for code review and commit messages
- Security checks and file validation
- Audit logging system
- Configuration management
