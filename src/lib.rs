//! Strongly-typed HTTP header parsing and construction for common headers
//!
//! This crate provides type-safe representations of frequently used HTTP headers,
//! with parsing from raw strings and serialization back to valid header values.
//!
//! No external dependencies — std only.
//!
//! # Supported Headers
//!
//! - [`ContentType`] — Content-Type with media type, subtype, and parameters
//! - [`CacheControl`] — Cache-Control directives with builder pattern
//! - [`Authorization`] — Bearer, Basic, and custom auth schemes
//! - [`Accept`] — Accept header with quality-based content negotiation
//! - [`SetCookie`] — Set-Cookie with all standard attributes
//! - [`ContentDisposition`] — Content-Disposition for inline/attachment
//! - [`ETag`] — ETag with strong and weak comparison
//!
//! # Example
//!
//! ```
//! use philiprehberger_typed_headers::{ContentType, CacheControl, Authorization};
//!
//! let ct = ContentType::parse("application/json; charset=utf-8").unwrap();
//! assert_eq!(ct.mime_type(), "application/json");
//! assert_eq!(ct.charset(), Some("utf-8"));
//!
//! let cc = CacheControl::new().no_cache().max_age(3600);
//! assert_eq!(cc.to_string(), "no-cache, max-age=3600");
//!
//! let auth = Authorization::bearer("my-token");
//! assert_eq!(auth.to_string(), "Bearer my-token");
//! ```

use std::fmt;

// ---------------------------------------------------------------------------
// Base64 encoder (standard alphabet, with padding)
// ---------------------------------------------------------------------------

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, HeaderError> {
    let input = input.trim_end_matches('=');
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u32 - b'A' as u32,
            'a'..='z' => c as u32 - b'a' as u32 + 26,
            '0'..='9' => c as u32 - b'0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => {
                return Err(HeaderError::InvalidValue(format!(
                    "invalid base64 character: {c}"
                )))
            }
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// HeaderError
// ---------------------------------------------------------------------------

/// Error type returned when a header string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// The header string has an invalid format.
    InvalidFormat(String),
    /// The header string contains an invalid value.
    InvalidValue(String),
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "invalid format: {msg}"),
            Self::InvalidValue(msg) => write!(f, "invalid value: {msg}"),
        }
    }
}

impl std::error::Error for HeaderError {}

// ---------------------------------------------------------------------------
// ContentType
// ---------------------------------------------------------------------------

/// A parsed Content-Type header.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::ContentType;
///
/// let ct = ContentType::parse("text/html; charset=utf-8").unwrap();
/// assert_eq!(ct.media_type(), "text");
/// assert_eq!(ct.subtype(), "html");
/// assert_eq!(ct.charset(), Some("utf-8"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentType {
    media_type: String,
    subtype: String,
    params: Vec<(String, String)>,
}

impl ContentType {
    /// Parse a Content-Type header value.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let s = s.trim();
        let (mime, params_str) = match s.find(';') {
            Some(i) => (s[..i].trim(), Some(s[i + 1..].trim())),
            None => (s, None),
        };
        let (media_type, subtype) = mime
            .split_once('/')
            .ok_or_else(|| HeaderError::InvalidFormat("missing '/' in Content-Type".into()))?;
        let media_type = media_type.trim().to_lowercase();
        let subtype = subtype.trim().to_lowercase();
        if media_type.is_empty() || subtype.is_empty() {
            return Err(HeaderError::InvalidFormat(
                "empty media type or subtype".into(),
            ));
        }
        let mut params = Vec::new();
        if let Some(ps) = params_str {
            for part in ps.split(';') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let (k, v) = part.split_once('=').ok_or_else(|| {
                    HeaderError::InvalidFormat(format!("invalid parameter: {part}"))
                })?;
                params.push((k.trim().to_lowercase(), v.trim().to_string()));
            }
        }
        Ok(Self {
            media_type,
            subtype,
            params,
        })
    }

    /// `application/json`
    pub fn json() -> Self {
        Self {
            media_type: "application".into(),
            subtype: "json".into(),
            params: Vec::new(),
        }
    }

    /// `text/html`
    pub fn html() -> Self {
        Self {
            media_type: "text".into(),
            subtype: "html".into(),
            params: Vec::new(),
        }
    }

    /// `text/plain`
    pub fn text() -> Self {
        Self {
            media_type: "text".into(),
            subtype: "plain".into(),
            params: Vec::new(),
        }
    }

    /// `application/x-www-form-urlencoded`
    pub fn form() -> Self {
        Self {
            media_type: "application".into(),
            subtype: "x-www-form-urlencoded".into(),
            params: Vec::new(),
        }
    }

    /// `multipart/form-data`
    pub fn multipart() -> Self {
        Self {
            media_type: "multipart".into(),
            subtype: "form-data".into(),
            params: Vec::new(),
        }
    }

    /// `application/xml`
    pub fn xml() -> Self {
        Self {
            media_type: "application".into(),
            subtype: "xml".into(),
            params: Vec::new(),
        }
    }

    /// `application/octet-stream`
    pub fn octet_stream() -> Self {
        Self {
            media_type: "application".into(),
            subtype: "octet-stream".into(),
            params: Vec::new(),
        }
    }

    /// Returns the media type (e.g. `"application"`).
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Returns the subtype (e.g. `"json"`).
    pub fn subtype(&self) -> &str {
        &self.subtype
    }

    /// Returns the `charset` parameter value if present.
    pub fn charset(&self) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == "charset")
            .map(|(_, v)| v.as_str())
    }

    /// Returns the MIME type string (e.g. `"application/json"`).
    pub fn mime_type(&self) -> String {
        format!("{}/{}", self.media_type, self.subtype)
    }

    /// Set or replace the `charset` parameter, returning `self`.
    ///
    /// # Example
    ///
    /// ```
    /// use philiprehberger_typed_headers::ContentType;
    ///
    /// let ct = ContentType::json().with_charset("utf-8");
    /// assert_eq!(ct.charset(), Some("utf-8"));
    /// assert_eq!(ct.to_string(), "application/json; charset=utf-8");
    /// ```
    pub fn with_charset(self, charset: &str) -> Self {
        self.with_param("charset", charset)
    }

    /// Set or replace an arbitrary parameter, returning `self`.
    ///
    /// Parameter names are stored lowercase to match parsing behavior.
    ///
    /// # Example
    ///
    /// ```
    /// use philiprehberger_typed_headers::ContentType;
    ///
    /// let ct = ContentType::multipart().with_param("boundary", "----abc123");
    /// assert_eq!(ct.to_string(), "multipart/form-data; boundary=----abc123");
    /// ```
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        let key_lower = key.to_lowercase();
        if let Some(existing) = self.params.iter_mut().find(|(k, _)| k == &key_lower) {
            existing.1 = value.to_string();
        } else {
            self.params.push((key_lower, value.to_string()));
        }
        self
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.media_type, self.subtype)?;
        for (k, v) in &self.params {
            write!(f, "; {k}={v}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CacheControl
// ---------------------------------------------------------------------------

/// A parsed Cache-Control header with builder pattern support.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::CacheControl;
///
/// let cc = CacheControl::new().no_cache().max_age(3600);
/// assert_eq!(cc.to_string(), "no-cache, max-age=3600");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheControl {
    /// `no-cache` directive
    pub no_cache: bool,
    /// `no-store` directive
    pub no_store: bool,
    /// `must-revalidate` directive
    pub must_revalidate: bool,
    /// `public` directive
    pub public: bool,
    /// `private` directive
    pub private: bool,
    /// `max-age` directive in seconds
    pub max_age: Option<u64>,
    /// `s-maxage` directive in seconds
    pub s_maxage: Option<u64>,
    /// `immutable` directive
    pub immutable: bool,
}

impl CacheControl {
    /// Parse a Cache-Control header value.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let mut cc = Self::new();
        for directive in s.split(',') {
            let directive = directive.trim().to_lowercase();
            if directive.is_empty() {
                continue;
            }
            match directive.as_str() {
                "no-cache" => cc.no_cache = true,
                "no-store" => cc.no_store = true,
                "must-revalidate" => cc.must_revalidate = true,
                "public" => cc.public = true,
                "private" => cc.private = true,
                "immutable" => cc.immutable = true,
                _ if directive.starts_with("max-age=") => {
                    let val = directive[8..].parse::<u64>().map_err(|_| {
                        HeaderError::InvalidValue(format!("invalid max-age: {directive}"))
                    })?;
                    cc.max_age = Some(val);
                }
                _ if directive.starts_with("s-maxage=") => {
                    let val = directive[9..].parse::<u64>().map_err(|_| {
                        HeaderError::InvalidValue(format!("invalid s-maxage: {directive}"))
                    })?;
                    cc.s_maxage = Some(val);
                }
                _ => {
                    // Ignore unknown directives for forward-compatibility
                }
            }
        }
        Ok(cc)
    }

    /// Create a new empty `CacheControl` (all directives off).
    pub fn new() -> Self {
        Self {
            no_cache: false,
            no_store: false,
            must_revalidate: false,
            public: false,
            private: false,
            max_age: None,
            s_maxage: None,
            immutable: false,
        }
    }

    /// Set the `no-cache` directive.
    pub fn no_cache(mut self) -> Self {
        self.no_cache = true;
        self
    }

    /// Set the `no-store` directive.
    pub fn no_store(mut self) -> Self {
        self.no_store = true;
        self
    }

    /// Set the `must-revalidate` directive.
    pub fn must_revalidate(mut self) -> Self {
        self.must_revalidate = true;
        self
    }

    /// Set the `public` directive.
    pub fn public(mut self) -> Self {
        self.public = true;
        self
    }

    /// Set the `private` directive.
    pub fn private(mut self) -> Self {
        self.private = true;
        self
    }

    /// Set the `max-age` directive in seconds.
    pub fn max_age(mut self, secs: u64) -> Self {
        self.max_age = Some(secs);
        self
    }

    /// Set the `s-maxage` directive in seconds.
    pub fn s_maxage(mut self, secs: u64) -> Self {
        self.s_maxage = Some(secs);
        self
    }

    /// Set the `immutable` directive.
    pub fn immutable(mut self) -> Self {
        self.immutable = true;
        self
    }
}

impl Default for CacheControl {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CacheControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.no_cache {
            parts.push("no-cache".into());
        }
        if self.no_store {
            parts.push("no-store".into());
        }
        if self.must_revalidate {
            parts.push("must-revalidate".into());
        }
        if self.public {
            parts.push("public".into());
        }
        if self.private {
            parts.push("private".into());
        }
        if let Some(ma) = self.max_age {
            parts.push(format!("max-age={ma}"));
        }
        if let Some(sm) = self.s_maxage {
            parts.push(format!("s-maxage={sm}"));
        }
        if self.immutable {
            parts.push("immutable".into());
        }
        write!(f, "{}", parts.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Authorization
// ---------------------------------------------------------------------------

/// A parsed Authorization header.
///
/// Supports Bearer tokens, Basic auth (with base64 encoding/decoding),
/// and arbitrary custom schemes.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::Authorization;
///
/// let auth = Authorization::bearer("my-token");
/// assert_eq!(auth.to_string(), "Bearer my-token");
///
/// let auth = Authorization::basic("user", "pass");
/// assert_eq!(auth.to_string(), "Basic dXNlcjpwYXNz");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authorization {
    /// Bearer token authentication.
    Bearer(String),
    /// Basic authentication with username and optional password.
    Basic {
        /// The username.
        username: String,
        /// The password (may be absent).
        password: Option<String>,
    },
    /// A custom authorization scheme.
    Custom {
        /// The scheme name.
        scheme: String,
        /// The credentials string.
        credentials: String,
    },
}

impl Authorization {
    /// Parse an Authorization header value.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let s = s.trim();
        let (scheme, credentials) = s.split_once(' ').ok_or_else(|| {
            HeaderError::InvalidFormat("missing scheme in Authorization header".into())
        })?;
        let credentials = credentials.trim();
        match scheme.to_lowercase().as_str() {
            "bearer" => Ok(Self::Bearer(credentials.to_string())),
            "basic" => {
                let decoded = base64_decode(credentials)?;
                let decoded_str = String::from_utf8(decoded).map_err(|_| {
                    HeaderError::InvalidValue("invalid UTF-8 in Basic credentials".into())
                })?;
                let (username, password) = match decoded_str.split_once(':') {
                    Some((u, p)) => (u.to_string(), Some(p.to_string())),
                    None => (decoded_str, None),
                };
                Ok(Self::Basic { username, password })
            }
            _ => Ok(Self::Custom {
                scheme: scheme.to_string(),
                credentials: credentials.to_string(),
            }),
        }
    }

    /// Create a Bearer authorization.
    pub fn bearer(token: &str) -> Self {
        Self::Bearer(token.to_string())
    }

    /// Create a Basic authorization, encoding the credentials to base64.
    pub fn basic(user: &str, pass: &str) -> Self {
        Self::Basic {
            username: user.to_string(),
            password: Some(pass.to_string()),
        }
    }

    /// Create a custom-scheme authorization (e.g. Digest, Hawk).
    ///
    /// # Example
    ///
    /// ```
    /// use philiprehberger_typed_headers::Authorization;
    ///
    /// let auth = Authorization::custom("Hawk", "id=\"abc\", mac=\"xyz\"");
    /// assert_eq!(auth.to_string(), "Hawk id=\"abc\", mac=\"xyz\"");
    /// ```
    pub fn custom(scheme: &str, credentials: &str) -> Self {
        Self::Custom {
            scheme: scheme.to_string(),
            credentials: credentials.to_string(),
        }
    }
}

impl fmt::Display for Authorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bearer(token) => write!(f, "Bearer {token}"),
            Self::Basic { username, password } => {
                let combined = match password {
                    Some(p) => format!("{username}:{p}"),
                    None => username.clone(),
                };
                write!(f, "Basic {}", base64_encode(combined.as_bytes()))
            }
            Self::Custom {
                scheme,
                credentials,
            } => write!(f, "{scheme} {credentials}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Accept / MediaRange
// ---------------------------------------------------------------------------

/// A single media range entry within an Accept header.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRange {
    media_type: String,
    subtype: String,
    quality: f32,
}

impl MediaRange {
    /// Returns the media type (e.g. `"text"`).
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Returns the subtype (e.g. `"html"`).
    pub fn subtype(&self) -> &str {
        &self.subtype
    }

    /// Returns the quality factor (0.0 to 1.0).
    pub fn quality(&self) -> f32 {
        self.quality
    }
}

impl fmt::Display for MediaRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.media_type, self.subtype)?;
        if (self.quality - 1.0).abs() > f32::EPSILON {
            write!(f, ";q={:.1}", self.quality)?;
        }
        Ok(())
    }
}

/// A parsed Accept header with content negotiation support.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::{Accept, ContentType};
///
/// let accept = Accept::parse("text/html, application/json;q=0.9").unwrap();
/// let available = vec![ContentType::json(), ContentType::html()];
/// let best = accept.best_match(&available).unwrap();
/// assert_eq!(best.mime_type(), "text/html");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Accept {
    types: Vec<MediaRange>,
}

impl Accept {
    /// Parse an Accept header value.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let mut types = Vec::new();
        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let mut quality: f32 = 1.0;
            let (mime_part, params) = match part.find(';') {
                Some(i) => (part[..i].trim(), Some(part[i + 1..].trim())),
                None => (part, None),
            };
            if let Some(params) = params {
                for p in params.split(';') {
                    let p = p.trim();
                    if let Some(qv) = p.strip_prefix("q=") {
                        quality = qv.parse::<f32>().map_err(|_| {
                            HeaderError::InvalidValue(format!("invalid quality value: {qv}"))
                        })?;
                    }
                }
            }
            let (media_type, subtype) = mime_part.split_once('/').ok_or_else(|| {
                HeaderError::InvalidFormat(format!("missing '/' in media range: {mime_part}"))
            })?;
            types.push(MediaRange {
                media_type: media_type.trim().to_lowercase(),
                subtype: subtype.trim().to_lowercase(),
                quality,
            });
        }
        if types.is_empty() {
            return Err(HeaderError::InvalidFormat("empty Accept header".into()));
        }
        Ok(Self { types })
    }

    /// Returns the media ranges in order.
    pub fn types(&self) -> &[MediaRange] {
        &self.types
    }

    /// Perform content negotiation: given a list of available content types,
    /// return the best match based on quality values.
    pub fn best_match<'a>(&self, available: &'a [ContentType]) -> Option<&'a ContentType> {
        let mut best: Option<(f32, &'a ContentType)> = None;
        for ct in available {
            let mut max_q: f32 = -1.0;
            for mr in &self.types {
                let type_match =
                    mr.media_type == "*" || mr.media_type == ct.media_type;
                let sub_match = mr.subtype == "*" || mr.subtype == ct.subtype;
                if type_match && sub_match && mr.quality > max_q {
                    max_q = mr.quality;
                }
            }
            if max_q >= 0.0 && (best.is_none() || max_q > best.unwrap().0) {
                best = Some((max_q, ct));
            }
        }
        best.map(|(_, ct)| ct)
    }
}

impl fmt::Display for Accept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.types.iter().map(|mr| mr.to_string()).collect();
        write!(f, "{}", parts.join(", "))
    }
}

// ---------------------------------------------------------------------------
// SetCookie / SameSite
// ---------------------------------------------------------------------------

/// The `SameSite` cookie attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SameSite {
    /// Cookies are only sent in first-party context.
    Strict,
    /// Cookies are sent on top-level navigations.
    Lax,
    /// Cookies are sent in all contexts (requires Secure).
    None,
}

impl fmt::Display for SameSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strict => write!(f, "Strict"),
            Self::Lax => write!(f, "Lax"),
            Self::None => write!(f, "None"),
        }
    }
}

/// A parsed Set-Cookie header with builder support.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::{SetCookie, SameSite};
///
/// let cookie = SetCookie::new("session", "abc123")
///     .secure()
///     .http_only()
///     .same_site(SameSite::Strict)
///     .path("/")
///     .max_age(3600);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCookie {
    name: String,
    value: String,
    path: Option<String>,
    domain: Option<String>,
    max_age: Option<u64>,
    expires: Option<String>,
    secure: bool,
    http_only: bool,
    partitioned: bool,
    same_site: Option<SameSite>,
}

impl SetCookie {
    /// Parse a Set-Cookie header value.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let s = s.trim();
        let mut parts = s.split(';');
        let name_value = parts
            .next()
            .ok_or_else(|| HeaderError::InvalidFormat("empty Set-Cookie".into()))?
            .trim();
        let (name, value) = name_value.split_once('=').ok_or_else(|| {
            HeaderError::InvalidFormat("missing '=' in cookie name=value".into())
        })?;
        let name = name.trim().to_string();
        let value = value.trim().to_string();
        if name.is_empty() {
            return Err(HeaderError::InvalidFormat("empty cookie name".into()));
        }
        let mut cookie = Self {
            name,
            value,
            path: None,
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            partitioned: false,
            same_site: None,
        };
        for attr in parts {
            let attr = attr.trim();
            if attr.is_empty() {
                continue;
            }
            let lower = attr.to_lowercase();
            if lower == "secure" {
                cookie.secure = true;
            } else if lower == "httponly" {
                cookie.http_only = true;
            } else if lower == "partitioned" {
                cookie.partitioned = true;
            } else if let Some(v) = strip_prefix_ci(attr, "expires=") {
                cookie.expires = Some(v.to_string());
            } else if let Some(v) = strip_prefix_ci(attr, "path=") {
                cookie.path = Some(v.to_string());
            } else if let Some(v) = strip_prefix_ci(attr, "domain=") {
                cookie.domain = Some(v.to_string());
            } else if let Some(v) = strip_prefix_ci(attr, "max-age=") {
                cookie.max_age = Some(v.parse::<u64>().map_err(|_| {
                    HeaderError::InvalidValue(format!("invalid Max-Age: {v}"))
                })?);
            } else if let Some(v) = strip_prefix_ci(attr, "samesite=") {
                cookie.same_site = Some(match v.to_lowercase().as_str() {
                    "strict" => SameSite::Strict,
                    "lax" => SameSite::Lax,
                    "none" => SameSite::None,
                    _ => {
                        return Err(HeaderError::InvalidValue(format!(
                            "invalid SameSite value: {v}"
                        )));
                    }
                });
            }
        }
        Ok(cookie)
    }

    /// Create a new Set-Cookie with the given name and value.
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            path: None,
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            partitioned: false,
            same_site: None,
        }
    }

    /// Returns the cookie name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the cookie value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the Path attribute.
    pub fn path(mut self, p: &str) -> Self {
        self.path = Some(p.to_string());
        self
    }

    /// Set the Domain attribute.
    pub fn domain(mut self, d: &str) -> Self {
        self.domain = Some(d.to_string());
        self
    }

    /// Set the Max-Age attribute in seconds.
    pub fn max_age(mut self, secs: u64) -> Self {
        self.max_age = Some(secs);
        self
    }

    /// Set the Secure flag.
    pub fn secure(mut self) -> Self {
        self.secure = true;
        self
    }

    /// Set the HttpOnly flag.
    pub fn http_only(mut self) -> Self {
        self.http_only = true;
        self
    }

    /// Set the SameSite attribute.
    pub fn same_site(mut self, ss: SameSite) -> Self {
        self.same_site = Some(ss);
        self
    }

    /// Set the Expires attribute. The value is an opaque HTTP-date string —
    /// no date validation is performed.
    pub fn expires(mut self, date: &str) -> Self {
        self.expires = Some(date.to_string());
        self
    }

    /// Returns the Expires attribute value if set.
    pub fn expires_value(&self) -> Option<&str> {
        self.expires.as_deref()
    }

    /// Set the Partitioned flag (CHIPS — partitioned cookie storage).
    pub fn partitioned(mut self) -> Self {
        self.partitioned = true;
        self
    }

    /// Returns true if the Partitioned flag is set.
    pub fn is_partitioned(&self) -> bool {
        self.partitioned
    }
}

impl fmt::Display for SetCookie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)?;
        if let Some(ref p) = self.path {
            write!(f, "; Path={p}")?;
        }
        if let Some(ref d) = self.domain {
            write!(f, "; Domain={d}")?;
        }
        if let Some(ma) = self.max_age {
            write!(f, "; Max-Age={ma}")?;
        }
        if let Some(ref e) = self.expires {
            write!(f, "; Expires={e}")?;
        }
        if self.secure {
            write!(f, "; Secure")?;
        }
        if self.http_only {
            write!(f, "; HttpOnly")?;
        }
        if let Some(ref ss) = self.same_site {
            write!(f, "; SameSite={ss}")?;
        }
        if self.partitioned {
            write!(f, "; Partitioned")?;
        }
        Ok(())
    }
}

/// Case-insensitive prefix strip helper.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// ContentDisposition
// ---------------------------------------------------------------------------

/// A parsed Content-Disposition header.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::ContentDisposition;
///
/// let cd = ContentDisposition::attachment("report.pdf");
/// assert_eq!(cd.to_string(), "attachment; filename=\"report.pdf\"");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDisposition {
    disposition: String,
    filename: Option<String>,
}

impl ContentDisposition {
    /// Parse a Content-Disposition header value.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let s = s.trim();
        let (disposition, rest) = match s.find(';') {
            Some(i) => (s[..i].trim(), Some(s[i + 1..].trim())),
            None => (s, None),
        };
        let disposition = disposition.to_lowercase();
        if disposition != "inline" && disposition != "attachment" {
            return Err(HeaderError::InvalidValue(format!(
                "unknown disposition type: {disposition}"
            )));
        }
        let mut filename = None;
        if let Some(rest) = rest {
            for part in rest.split(';') {
                let part = part.trim();
                if let Some(v) = strip_prefix_ci(part, "filename=") {
                    let v = v.trim_matches('"');
                    filename = Some(v.to_string());
                }
            }
        }
        Ok(Self {
            disposition,
            filename,
        })
    }

    /// Create an inline Content-Disposition.
    pub fn inline() -> Self {
        Self {
            disposition: "inline".into(),
            filename: None,
        }
    }

    /// Create an attachment Content-Disposition with a filename.
    pub fn attachment(filename: &str) -> Self {
        Self {
            disposition: "attachment".into(),
            filename: Some(filename.to_string()),
        }
    }

    /// Returns the disposition type (`"inline"` or `"attachment"`).
    pub fn disposition(&self) -> &str {
        &self.disposition
    }

    /// Returns the filename if present.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }
}

impl fmt::Display for ContentDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.disposition)?;
        if let Some(ref name) = self.filename {
            write!(f, "; filename=\"{name}\"")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ETag
// ---------------------------------------------------------------------------

/// A parsed ETag header with strong and weak comparison.
///
/// # Example
///
/// ```
/// use philiprehberger_typed_headers::ETag;
///
/// let strong = ETag::parse("\"abc123\"").unwrap();
/// let weak = ETag::parse("W/\"abc123\"").unwrap();
/// assert!(strong.weak_match(&weak));
/// assert!(!strong.strong_match(&weak));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ETag {
    value: String,
    weak: bool,
}

impl ETag {
    /// Parse an ETag header value.
    ///
    /// Accepts both strong (`"value"`) and weak (`W/"value"`) forms.
    pub fn parse(s: &str) -> Result<Self, HeaderError> {
        let s = s.trim();
        let (weak, quoted) = if let Some(rest) = s.strip_prefix("W/") {
            (true, rest)
        } else {
            (false, s)
        };
        if !quoted.starts_with('"') || !quoted.ends_with('"') || quoted.len() < 2 {
            return Err(HeaderError::InvalidFormat(
                "ETag value must be quoted".into(),
            ));
        }
        let value = quoted[1..quoted.len() - 1].to_string();
        Ok(Self { value, weak })
    }

    /// Create a new strong ETag.
    pub fn strong(value: &str) -> Self {
        Self {
            value: value.to_string(),
            weak: false,
        }
    }

    /// Create a new weak ETag.
    pub fn weak(value: &str) -> Self {
        Self {
            value: value.to_string(),
            weak: true,
        }
    }

    /// Returns the tag value (without quotes or W/ prefix).
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns true if this is a weak ETag.
    pub fn is_weak(&self) -> bool {
        self.weak
    }

    /// Strong comparison: both must be strong and have the same value.
    pub fn strong_match(&self, other: &ETag) -> bool {
        !self.weak && !other.weak && self.value == other.value
    }

    /// Weak comparison: values must match regardless of weakness.
    pub fn weak_match(&self, other: &ETag) -> bool {
        self.value == other.value
    }
}

impl fmt::Display for ETag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.weak {
            write!(f, "W/\"{}\"", self.value)
        } else {
            write!(f, "\"{}\"", self.value)
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- ContentType --------------------------------------------------------

    #[test]
    fn content_type_parse_with_charset() {
        let ct = ContentType::parse("application/json; charset=utf-8").unwrap();
        assert_eq!(ct.media_type(), "application");
        assert_eq!(ct.subtype(), "json");
        assert_eq!(ct.charset(), Some("utf-8"));
        assert_eq!(ct.mime_type(), "application/json");
    }

    #[test]
    fn content_type_parse_simple() {
        let ct = ContentType::parse("text/plain").unwrap();
        assert_eq!(ct.media_type(), "text");
        assert_eq!(ct.subtype(), "plain");
        assert_eq!(ct.charset(), None);
    }

    #[test]
    fn content_type_constructors() {
        assert_eq!(ContentType::json().mime_type(), "application/json");
        assert_eq!(ContentType::html().mime_type(), "text/html");
        assert_eq!(ContentType::text().mime_type(), "text/plain");
        assert_eq!(
            ContentType::form().mime_type(),
            "application/x-www-form-urlencoded"
        );
        assert_eq!(ContentType::multipart().mime_type(), "multipart/form-data");
        assert_eq!(ContentType::xml().mime_type(), "application/xml");
        assert_eq!(
            ContentType::octet_stream().mime_type(),
            "application/octet-stream"
        );
    }

    #[test]
    fn content_type_display_roundtrip() {
        let original = "application/json; charset=utf-8";
        let ct = ContentType::parse(original).unwrap();
        let displayed = ct.to_string();
        let reparsed = ContentType::parse(&displayed).unwrap();
        assert_eq!(ct, reparsed);
    }

    #[test]
    fn content_type_invalid_format() {
        assert!(ContentType::parse("invalid").is_err());
        assert!(ContentType::parse("/json").is_err());
        assert!(ContentType::parse("application/").is_err());
    }

    // -- CacheControl -------------------------------------------------------

    #[test]
    fn cache_control_parse() {
        let cc = CacheControl::parse("no-cache, max-age=3600").unwrap();
        assert!(cc.no_cache);
        assert!(!cc.no_store);
        assert_eq!(cc.max_age, Some(3600));
    }

    #[test]
    fn cache_control_parse_full() {
        let cc = CacheControl::parse(
            "no-cache, no-store, must-revalidate, public, private, max-age=60, s-maxage=120, immutable",
        )
        .unwrap();
        assert!(cc.no_cache);
        assert!(cc.no_store);
        assert!(cc.must_revalidate);
        assert!(cc.public);
        assert!(cc.private);
        assert_eq!(cc.max_age, Some(60));
        assert_eq!(cc.s_maxage, Some(120));
        assert!(cc.immutable);
    }

    #[test]
    fn cache_control_builder() {
        let cc = CacheControl::new().no_cache().no_store().max_age(3600);
        assert!(cc.no_cache);
        assert!(cc.no_store);
        assert_eq!(cc.max_age, Some(3600));
    }

    #[test]
    fn cache_control_display() {
        let cc = CacheControl::new().no_cache().max_age(3600);
        assert_eq!(cc.to_string(), "no-cache, max-age=3600");
    }

    #[test]
    fn cache_control_display_empty() {
        let cc = CacheControl::new();
        assert_eq!(cc.to_string(), "");
    }

    // -- Authorization ------------------------------------------------------

    #[test]
    fn authorization_parse_bearer() {
        let auth = Authorization::parse("Bearer my-secret-token").unwrap();
        assert_eq!(auth, Authorization::Bearer("my-secret-token".into()));
    }

    #[test]
    fn authorization_parse_basic() {
        // "user:pass" -> base64 = "dXNlcjpwYXNz"
        let auth = Authorization::parse("Basic dXNlcjpwYXNz").unwrap();
        match auth {
            Authorization::Basic { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, Some("pass".into()));
            }
            _ => panic!("expected Basic"),
        }
    }

    #[test]
    fn authorization_bearer_display() {
        let auth = Authorization::bearer("tok123");
        assert_eq!(auth.to_string(), "Bearer tok123");
    }

    #[test]
    fn authorization_basic_display_roundtrip() {
        let auth = Authorization::basic("admin", "secret");
        let displayed = auth.to_string();
        let reparsed = Authorization::parse(&displayed).unwrap();
        assert_eq!(reparsed, auth);
    }

    #[test]
    fn authorization_custom() {
        let auth = Authorization::parse("Digest realm=\"test\"").unwrap();
        match auth {
            Authorization::Custom {
                scheme,
                credentials,
            } => {
                assert_eq!(scheme, "Digest");
                assert_eq!(credentials, "realm=\"test\"");
            }
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn authorization_invalid_format() {
        assert!(Authorization::parse("").is_err());
        assert!(Authorization::parse("BearerWithoutSpace").is_err());
    }

    // -- Accept -------------------------------------------------------------

    #[test]
    fn accept_parse_simple() {
        let accept = Accept::parse("text/html").unwrap();
        assert_eq!(accept.types().len(), 1);
        assert_eq!(accept.types()[0].media_type(), "text");
        assert_eq!(accept.types()[0].subtype(), "html");
        assert!((accept.types()[0].quality() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn accept_parse_with_quality() {
        let accept =
            Accept::parse("text/html, application/json;q=0.9, */*;q=0.1").unwrap();
        assert_eq!(accept.types().len(), 3);
        assert!((accept.types()[1].quality() - 0.9).abs() < f32::EPSILON);
        assert!((accept.types()[2].quality() - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn accept_best_match() {
        let accept =
            Accept::parse("text/html, application/json;q=0.9").unwrap();
        let available = vec![ContentType::json(), ContentType::html()];
        let best = accept.best_match(&available).unwrap();
        assert_eq!(best.mime_type(), "text/html");
    }

    #[test]
    fn accept_best_match_wildcard() {
        let accept = Accept::parse("*/*").unwrap();
        let available = vec![ContentType::json()];
        let best = accept.best_match(&available).unwrap();
        assert_eq!(best.mime_type(), "application/json");
    }

    #[test]
    fn accept_best_match_no_match() {
        let accept = Accept::parse("image/png").unwrap();
        let available = vec![ContentType::json(), ContentType::html()];
        assert!(accept.best_match(&available).is_none());
    }

    #[test]
    fn accept_display() {
        let accept =
            Accept::parse("text/html, application/json;q=0.9").unwrap();
        let displayed = accept.to_string();
        assert!(displayed.contains("text/html"));
        assert!(displayed.contains("application/json;q=0.9"));
    }

    #[test]
    fn accept_invalid() {
        assert!(Accept::parse("").is_err());
        assert!(Accept::parse("invalid").is_err());
    }

    // -- SetCookie ----------------------------------------------------------

    #[test]
    fn set_cookie_parse_full() {
        let cookie = SetCookie::parse(
            "session=abc123; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600",
        )
        .unwrap();
        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.value(), "abc123");
        assert!(cookie.secure);
        assert!(cookie.http_only);
        assert_eq!(cookie.same_site, Some(SameSite::Strict));
        assert_eq!(cookie.path, Some("/".into()));
        assert_eq!(cookie.max_age, Some(3600));
    }

    #[test]
    fn set_cookie_parse_simple() {
        let cookie = SetCookie::parse("id=42").unwrap();
        assert_eq!(cookie.name(), "id");
        assert_eq!(cookie.value(), "42");
        assert!(!cookie.secure);
        assert!(!cookie.http_only);
    }

    #[test]
    fn set_cookie_builder() {
        let cookie = SetCookie::new("token", "xyz")
            .secure()
            .http_only()
            .path("/api")
            .domain("example.com")
            .max_age(7200)
            .same_site(SameSite::Lax);
        assert_eq!(cookie.name(), "token");
        assert!(cookie.secure);
        assert!(cookie.http_only);
        assert_eq!(cookie.path, Some("/api".into()));
        assert_eq!(cookie.domain, Some("example.com".into()));
        assert_eq!(cookie.max_age, Some(7200));
        assert_eq!(cookie.same_site, Some(SameSite::Lax));
    }

    #[test]
    fn set_cookie_display() {
        let cookie = SetCookie::new("a", "b")
            .path("/")
            .secure()
            .http_only()
            .same_site(SameSite::None)
            .max_age(60);
        let s = cookie.to_string();
        assert!(s.starts_with("a=b"));
        assert!(s.contains("Path=/"));
        assert!(s.contains("Secure"));
        assert!(s.contains("HttpOnly"));
        assert!(s.contains("SameSite=None"));
        assert!(s.contains("Max-Age=60"));
    }

    #[test]
    fn set_cookie_invalid() {
        assert!(SetCookie::parse("").is_err());
        assert!(SetCookie::parse("noequals").is_err());
        assert!(SetCookie::parse("=value").is_err());
    }

    // -- ContentDisposition -------------------------------------------------

    #[test]
    fn content_disposition_parse_attachment() {
        let cd =
            ContentDisposition::parse("attachment; filename=\"report.pdf\"").unwrap();
        assert_eq!(cd.disposition(), "attachment");
        assert_eq!(cd.filename(), Some("report.pdf"));
    }

    #[test]
    fn content_disposition_parse_inline() {
        let cd = ContentDisposition::parse("inline").unwrap();
        assert_eq!(cd.disposition(), "inline");
        assert_eq!(cd.filename(), None);
    }

    #[test]
    fn content_disposition_display() {
        let cd = ContentDisposition::attachment("data.csv");
        assert_eq!(cd.to_string(), "attachment; filename=\"data.csv\"");
    }

    #[test]
    fn content_disposition_inline_display() {
        let cd = ContentDisposition::inline();
        assert_eq!(cd.to_string(), "inline");
    }

    #[test]
    fn content_disposition_invalid() {
        assert!(ContentDisposition::parse("download").is_err());
    }

    // -- ETag ---------------------------------------------------------------

    #[test]
    fn etag_parse_strong() {
        let etag = ETag::parse("\"abc123\"").unwrap();
        assert_eq!(etag.value(), "abc123");
        assert!(!etag.is_weak());
    }

    #[test]
    fn etag_parse_weak() {
        let etag = ETag::parse("W/\"abc123\"").unwrap();
        assert_eq!(etag.value(), "abc123");
        assert!(etag.is_weak());
    }

    #[test]
    fn etag_strong_match() {
        let a = ETag::strong("v1");
        let b = ETag::strong("v1");
        let c = ETag::weak("v1");
        assert!(a.strong_match(&b));
        assert!(!a.strong_match(&c));
    }

    #[test]
    fn etag_weak_match() {
        let strong = ETag::strong("v1");
        let weak = ETag::weak("v1");
        assert!(strong.weak_match(&weak));
        assert!(weak.weak_match(&strong));
    }

    #[test]
    fn etag_no_match() {
        let a = ETag::strong("v1");
        let b = ETag::strong("v2");
        assert!(!a.strong_match(&b));
        assert!(!a.weak_match(&b));
    }

    #[test]
    fn etag_display() {
        assert_eq!(ETag::strong("abc").to_string(), "\"abc\"");
        assert_eq!(ETag::weak("abc").to_string(), "W/\"abc\"");
    }

    #[test]
    fn etag_invalid() {
        assert!(ETag::parse("abc").is_err());
        assert!(ETag::parse("\"").is_err());
    }

    // -- New v0.2.0 surface -------------------------------------------------

    #[test]
    fn content_type_with_charset_adds_param() {
        let ct = ContentType::json().with_charset("utf-8");
        assert_eq!(ct.charset(), Some("utf-8"));
        assert_eq!(ct.to_string(), "application/json; charset=utf-8");
    }

    #[test]
    fn content_type_with_charset_replaces_existing() {
        let ct = ContentType::parse("text/html; charset=iso-8859-1").unwrap();
        let updated = ct.with_charset("utf-8");
        assert_eq!(updated.charset(), Some("utf-8"));
        // No duplicate charset parameter
        assert_eq!(updated.to_string(), "text/html; charset=utf-8");
    }

    #[test]
    fn content_type_with_param_arbitrary() {
        let ct = ContentType::multipart().with_param("boundary", "----abc123");
        assert_eq!(ct.to_string(), "multipart/form-data; boundary=----abc123");
    }

    #[test]
    fn authorization_custom_constructor() {
        let auth = Authorization::custom("Hawk", "id=\"abc\"");
        match &auth {
            Authorization::Custom { scheme, credentials } => {
                assert_eq!(scheme, "Hawk");
                assert_eq!(credentials, "id=\"abc\"");
            }
            _ => panic!("expected Custom"),
        }
        assert_eq!(auth.to_string(), "Hawk id=\"abc\"");
    }

    #[test]
    fn set_cookie_expires_roundtrip() {
        let cookie = SetCookie::new("session", "abc")
            .expires("Wed, 21 Oct 2026 07:28:00 GMT");
        let s = cookie.to_string();
        assert!(s.contains("Expires=Wed, 21 Oct 2026 07:28:00 GMT"));

        let parsed = SetCookie::parse(&s).unwrap();
        assert_eq!(
            parsed.expires_value(),
            Some("Wed, 21 Oct 2026 07:28:00 GMT")
        );
    }

    #[test]
    fn set_cookie_partitioned_roundtrip() {
        let cookie = SetCookie::new("__Host-session", "xyz")
            .secure()
            .http_only()
            .same_site(SameSite::None)
            .partitioned();
        let s = cookie.to_string();
        assert!(s.contains("Partitioned"));

        let parsed = SetCookie::parse(&s).unwrap();
        assert!(parsed.is_partitioned());
        assert!(parsed.secure);
    }

    // -- Base64 roundtrip ---------------------------------------------------

    #[test]
    fn base64_roundtrip() {
        let input = "Hello, World!";
        let encoded = base64_encode(input.as_bytes());
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), input);
    }

    #[test]
    fn base64_known_values() {
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }
}
