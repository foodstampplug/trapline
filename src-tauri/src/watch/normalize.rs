use once_cell::sync::Lazy;
use regex::Regex;

// Cache-busting hash segments: `app.4f3a2b1c.js`, `main-AbC12345.js`, `chunk.12ab34cd.chunk.js`
static HASH_SEG: Lazy<Regex> = Lazy::new(|| Regex::new(r"[._-][0-9a-fA-F]{8,}").unwrap());
// Long digit runs (build numbers / timestamps): `bundle.16998877.js`
static NUM_SEG: Lazy<Regex> = Lazy::new(|| Regex::new(r"[._-]\d{6,}").unwrap());

/// Reduce a JS URL to a stable identity that survives cache-busting hashes,
/// so `app.4f3a2b.js` and `app.9c8d7e.js` are recognised as the SAME asset.
pub fn logical_url(raw: &str) -> String {
    // Strip a trailing query string first (?v=hash is a cache-bust too).
    let base = raw.split(['?', '#']).next().unwrap_or(raw);
    let a = HASH_SEG.replace_all(base, "");
    let b = NUM_SEG.replace_all(&a, "");
    b.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_hash() {
        assert_eq!(
            logical_url("https://x.com/static/app.4f3a2b1c.js"),
            "https://x.com/static/app.js"
        );
    }

    #[test]
    fn strips_query_cachebust() {
        assert_eq!(
            logical_url("https://x.com/main.js?v=99887766"),
            "https://x.com/main.js"
        );
    }

    #[test]
    fn keeps_meaningful_version() {
        // /api/v2/ style version markers are short and must be preserved
        assert_eq!(logical_url("https://x.com/v2/app.js"), "https://x.com/v2/app.js");
    }
}
