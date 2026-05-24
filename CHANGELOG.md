# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `contasty <PATH>` CLI: gitignore-aware walker, tree-sitter parsing, Markdown
  rendering. Strips Rust `fn` bodies, keeps signatures, types, and doc comments.
- `Language` registry: adding a language is grammar dependency + sibling module
  + one tree-sitter query.

### Removed

- `greet` placeholder library function and `log` dependency.
