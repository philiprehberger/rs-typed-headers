# Changelog

## 0.2.0 (2026-04-27)

- Add `ContentType::with_charset()` and `ContentType::with_param()` builders
- Add `Authorization::custom(scheme, credentials)` constructor
- Add `SetCookie::expires()` builder + parsing of the `Expires=` attribute
- Add `SetCookie::partitioned()` builder + parsing of the `Partitioned` attribute (CHIPS)

## 0.1.4 (2026-03-31)

- Standardize README to 3-badge format with emoji Support section
- Update CI checkout action to v5 for Node.js 24 compatibility

## 0.1.3 (2026-03-27)

- Add GitHub issue templates, PR template, and dependabot configuration
- Update README badges and add Support section

## 0.1.2 (2026-03-22)

- Fix README and CHANGELOG compliance

## 0.1.1 (2026-03-20)

- Re-release with registry token configured

## 0.1.0 (2026-03-20)

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
