# rs-typed-headers

[![CI](https://github.com/philiprehberger/rs-typed-headers/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-typed-headers/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-typed-headers.svg)](https://crates.io/crates/philiprehberger-typed-headers)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-typed-headers)](https://github.com/philiprehberger/rs-typed-headers/commits/main)

Strongly-typed HTTP header parsing and construction for common headers

## Installation

```toml
[dependencies]
philiprehberger-typed-headers = "0.2.0"
```

## Usage

```rust
use philiprehberger_typed_headers::ContentType;

let ct = ContentType::parse("application/json; charset=utf-8").unwrap();
assert_eq!(ct.mime_type(), "application/json");
assert_eq!(ct.charset(), Some("utf-8"));

// Convenience constructors
let json = ContentType::json();
let html = ContentType::html();

// Builder helpers for charset and arbitrary parameters
let json_utf8 = ContentType::json().with_charset("utf-8");
let multipart = ContentType::multipart().with_param("boundary", "----abc123");
```

### Cache-Control

```rust
use philiprehberger_typed_headers::CacheControl;

// Parse from string
let cc = CacheControl::parse("no-cache, max-age=3600").unwrap();

// Builder pattern
let cc = CacheControl::new()
    .no_store()
    .must_revalidate()
    .max_age(86400);
assert_eq!(cc.to_string(), "no-store, must-revalidate, max-age=86400");
```

### Authorization

```rust
use philiprehberger_typed_headers::Authorization;

let auth = Authorization::bearer("my-token");
assert_eq!(auth.to_string(), "Bearer my-token");

// Basic auth (base64 encoded automatically)
let auth = Authorization::basic("user", "pass");
let parsed = Authorization::parse(&auth.to_string()).unwrap();
```

### Accept (Content Negotiation)

```rust
use philiprehberger_typed_headers::{Accept, ContentType};

let accept = Accept::parse("text/html, application/json;q=0.9, */*;q=0.1").unwrap();
let available = vec![ContentType::json(), ContentType::html()];
let best = accept.best_match(&available).unwrap();
assert_eq!(best.mime_type(), "text/html");
```

### Set-Cookie

```rust
use philiprehberger_typed_headers::{SetCookie, SameSite};

let cookie = SetCookie::new("session", "abc123")
    .secure()
    .http_only()
    .same_site(SameSite::Strict)
    .path("/")
    .max_age(3600);

// Expires attribute and Partitioned (CHIPS) flag
let partitioned = SetCookie::new("__Host-session", "xyz")
    .secure()
    .http_only()
    .same_site(SameSite::None)
    .expires("Wed, 21 Oct 2026 07:28:00 GMT")
    .partitioned();
```

### Content-Disposition

```rust
use philiprehberger_typed_headers::ContentDisposition;

let cd = ContentDisposition::attachment("report.pdf");
assert_eq!(cd.to_string(), "attachment; filename=\"report.pdf\"");

let parsed = ContentDisposition::parse("attachment; filename=\"data.csv\"").unwrap();
assert_eq!(parsed.filename(), Some("data.csv"));
```

### ETag

```rust
use philiprehberger_typed_headers::ETag;

let strong = ETag::parse("\"abc123\"").unwrap();
let weak = ETag::parse("W/\"abc123\"").unwrap();

assert!(strong.weak_match(&weak));   // values match
assert!(!strong.strong_match(&weak)); // one is weak
```

## API

| Type | Description |
|------|-------------|
| `ContentType` | Content-Type with media type, subtype, and parameters |
| `ContentType::parse(s)` | Parse a Content-Type header string |
| `ContentType::json()` | `application/json` |
| `ContentType::html()` | `text/html` |
| `ContentType::text()` | `text/plain` |
| `ContentType::form()` | `application/x-www-form-urlencoded` |
| `ContentType::multipart()` | `multipart/form-data` |
| `ContentType::xml()` | `application/xml` |
| `ContentType::octet_stream()` | `application/octet-stream` |
| `ContentType::with_charset(charset)` | Set or replace the `charset` parameter |
| `ContentType::with_param(key, value)` | Set or replace an arbitrary parameter |
| `CacheControl` | Cache-Control directives with builder pattern |
| `CacheControl::parse(s)` | Parse a Cache-Control header string |
| `CacheControl::new()` | Create empty cache control (builder start) |
| `Authorization` | Bearer, Basic, and custom auth schemes |
| `Authorization::parse(s)` | Parse an Authorization header string |
| `Authorization::bearer(token)` | Create a Bearer authorization |
| `Authorization::basic(user, pass)` | Create a Basic authorization (base64 encoded) |
| `Authorization::custom(scheme, credentials)` | Create a custom-scheme authorization |
| `Accept` | Accept header with quality-based content negotiation |
| `Accept::parse(s)` | Parse an Accept header string |
| `Accept::best_match(available)` | Content negotiation against available types |
| `SetCookie` | Set-Cookie with all standard attributes |
| `SetCookie::parse(s)` | Parse a Set-Cookie header string |
| `SetCookie::new(name, value)` | Create a new cookie (builder start) |
| `SetCookie::expires(date)` | Set the Expires attribute (opaque date string) |
| `SetCookie::partitioned()` | Set the Partitioned flag (CHIPS) |
| `ContentDisposition` | Content-Disposition for inline/attachment |
| `ContentDisposition::parse(s)` | Parse a Content-Disposition header string |
| `ContentDisposition::attachment(filename)` | Create an attachment disposition |
| `ContentDisposition::inline()` | Create an inline disposition |
| `ETag` | ETag with strong and weak comparison |
| `ETag::parse(s)` | Parse an ETag header string |
| `ETag::strong_match(other)` | Compare ETags using strong comparison |
| `ETag::weak_match(other)` | Compare ETags using weak comparison |
| `HeaderError` | Error type for parse failures |
| `SameSite` | SameSite cookie attribute enum |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-typed-headers)

🐛 [Report issues](https://github.com/philiprehberger/rs-typed-headers/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-typed-headers/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
