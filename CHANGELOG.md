# Changelog

All notable changes to this project should be recorded here.

This project does not yet publish formal releases. Until releases are cut, use
the `Unreleased` section to group user-visible changes.

## Unreleased

### Added

- Structured CLI argument parsing with `clap`.
- Typed JSON export and diagnostics serialization via `serde_json`.
- CLI integration tests for help, usage errors, default demo behavior, JSON
  export, and diagnostics error output.
- Contributor guidance and reproduction-oriented documentation notes.

### Changed

- Graph export and diagnostics JSON generation now use typed schemas instead of
  manually assembled JSON strings.

