# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Breaking Changes
- Command to start master process changed from `bindrs master` to `bindrs run`

## [0.0.4] - 2017-02-27
### Changed
- Rearranged project structure

### Fixed
- Skip attempting to send new/changed files to remote if they no longer exist
- Aquire lock earlier when local changes are made
- Force rsync changes, ignore IO errors using `--ignore-errors`

## [0.0.3] - 2017-02-02
### Changed
- Use action, not file system check when skipping locally created folders in watcher

### Fixed
- Check for remote bindrs binary, was returning false-positive
- Bad full path generation causing a crash
- Removed unwraps in favour of expects and matches

## [0.0.2] - 2017-01-30
### Added
- Check for remote folder before syncing starts
- Pass through verbose flag to child process

### Fixed
- Slave starts log in slave mode, not master mode
- Wait between last error log and exit, allows log to flush
- Allow release builds to log debug lines (needed for verbose mode)

[Unreleased]: https://github.com/daveallie/bindrs/compare/v0.0.4...HEAD
[0.0.4]: https://github.com/daveallie/bindrs/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/daveallie/bindrs/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/daveallie/bindrs/compare/v0.0.1...v0.0.2
