# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.1 (2026-03-20)

- Re-release with registry token configured

## [0.1.0] - 2026-03-20

### Added

- `ContentType` struct with parsing and common constructors (json, html, text, form, multipart, xml, octet_stream)
- `CacheControl` struct with parsing and builder pattern
- `Authorization` enum with Bearer, Basic, and Custom variants
- `Accept` struct with quality-based content negotiation via `best_match`
- `SetCookie` struct with full attribute parsing and builder pattern
- `ContentDisposition` struct for inline/attachment headers
- `ETag` struct with strong and weak matching
- `HeaderError` enum for parse error reporting
- Display implementations for all header types
- Inline base64 encoding for Basic auth (no external dependencies)

[0.1.0]: https://github.com/philiprehberger/rs-typed-headers/releases/tag/v0.1.0
