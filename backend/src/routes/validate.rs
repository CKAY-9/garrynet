//! Request-body validation shared by the routes. Each helper trims its input and returns
//! a 400 naming the field when it's invalid.

use crate::error::ApiError;

pub const MAX_URL: usize = 2048;

pub fn bad(message: String) -> ApiError {
    ApiError::BadRequest(message)
}

/// Trimmed text of at most `max` characters; may be empty.
pub fn text(field: &str, value: &str, max: usize) -> Result<String, ApiError> {
    let value = value.trim();
    if value.chars().count() > max {
        return Err(bad(format!("{field} must be at most {max} characters")));
    }
    Ok(value.to_owned())
}

/// Like [`text`], but must not be blank.
pub fn required_text(field: &str, value: &str, max: usize) -> Result<String, ApiError> {
    let value = text(field, value, max)?;
    if value.is_empty() {
        return Err(bad(format!("{field} is required")));
    }
    Ok(value)
}

/// An absolute http(s) URL, normalized (quotes, spaces etc. percent-encoded) so it's safe
/// to drop into an `href` or CSS `url()`.
pub fn url(field: &str, value: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.len() > MAX_URL {
        return Err(bad(format!("{field} must be at most {MAX_URL} characters")));
    }
    match url::Url::parse(value) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => Ok(parsed.into()),
        _ => Err(bad(format!("{field} must be an http(s) URL, got {value:?}"))),
    }
}

/// Like [`url`], but missing or blank means `None`.
pub fn optional_url(field: &str, value: Option<&str>) -> Result<Option<String>, ApiError> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(value) => url(field, value).map(Some),
    }
}

/// A hex color: `#rgb`, `#rrggbb` or `#rrggbbaa`, lowercased.
pub fn hex_color(field: &str, value: &str) -> Result<String, ApiError> {
    let value = value.trim();
    let valid = value
        .strip_prefix('#')
        .is_some_and(|hex| matches!(hex.len(), 3 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit()));
    if !valid {
        return Err(bad(format!("{field} must be a hex color like #3b82f6, got {value:?}")));
    }
    Ok(value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(hex_color("c", " #3B82F6 ").unwrap(), "#3b82f6");
        assert!(hex_color("c", "#fff").is_ok());
        assert!(hex_color("c", "#ffffff80").is_ok());
        assert!(hex_color("c", "3b82f6").is_err());
        assert!(hex_color("c", "#3b82f").is_err());
        assert!(hex_color("c", "#ggg").is_err());
    }

    #[test]
    fn urls() {
        assert_eq!(optional_url("u", Some("  ")).unwrap(), None);
        assert!(url("u", "javascript:alert(1)").is_err());
        assert!(url("u", "https://example.com/a b").unwrap().contains("%20"));
    }
}
