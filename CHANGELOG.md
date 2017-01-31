# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Changed
- Use action, not file system check when skipping locally created folders in watcher

### Fixed
- Check for remote bindrs binary, was returning false-positive
- Bad full path generation causing a crash

## [0.0.2] - 2015-12-03
### Added
- Check for remote folder before syncing starts
- Pass through verbose flag to child process

### Fixed
- Slave starts log in slave mode, not master mode
- Wait between last error log and exit, allows log to flush
- Allow release builds to log debug lines (needed for verbose mode)

[Unreleased]: https://github.com/daveallie/bindrs/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/daveallie/bindrs/compare/v0.0.1...v0.0.2
