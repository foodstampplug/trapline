use anyhow::{anyhow, Result};
use base64::Engine as _;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use url::Url;

/// One reconstructed original source file from a source map.
pub struct SourceFile {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMap {
    #[serde(default)]
    sources: Vec<String>,
    #[serde(default)]
    sources_content: Vec<Option<String>>,
}

// //# sourceMappingURL=...   or   //@ sourceMappingURL=...
static SM_REF: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?m)//[#@]\s*sourceMappingURL=([^\s'"]+)"#).unwrap());

/// The raw `sourceMappingURL` value from a bundle (a relative/absolute URL or a
/// `data:` URI), if the bundle declares one.
pub fn find_ref(bundle_text: &str) -> Option<String> {
    SM_REF
        .captures(bundle_text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

/// Resolve a non-data sourceMappingURL against the bundle URL to an absolute URL.
pub fn resolve_url(bundle_url: &str, sm_ref: &str) -> Option<String> {
    if sm_ref.starts_with("data:") {
        return None;
    }
    Url::parse(bundle_url).ok()?.join(sm_ref).ok().map(|u| u.to_string())
}

/// `<bundle>.map` — the convention used when the comment has been stripped but
/// the map is still deployed.
pub fn heuristic_map_url(bundle_url: &str) -> String {
    let base = bundle_url.split(['?', '#']).next().unwrap_or(bundle_url);
    format!("{base}.map")
}

/// Decode an inline `data:` source map URI to its JSON text.
pub fn decode_data_uri(sm_ref: &str) -> Option<String> {
    let comma = sm_ref.find(',')?;
    let meta = &sm_ref[..comma];
    let body = &sm_ref[comma + 1..];
    if meta.contains("base64") {
        let cleaned: String = body.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = base64::engine::general_purpose::STANDARD.decode(cleaned).ok()?;
        String::from_utf8(bytes).ok()
    } else {
        // percent-encoded or raw JSON
        Some(body.replace("%7B", "{").replace("%7D", "}").replace("%22", "\""))
    }
}

fn clean_path(s: &str) -> String {
    let mut p = s.to_string();
    for pre in ["webpack://", "webpack:///", "rollup://", "vite://"] {
        if let Some(rest) = p.strip_prefix(pre) {
            p = rest.to_string();
        }
    }
    // strip a leading namespace segment before "/./"  (e.g. "app-name/./src/x" -> "src/x")
    if let Some(idx) = p.find("/./") {
        p = p[idx + 3..].to_string();
    }
    p.trim_start_matches("./").trim_start_matches('/').to_string()
}

fn is_vendor(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("node_modules")
        || p.contains("/webpack/runtime")
        || p.starts_with("webpack/bootstrap")
        || p.contains("/vendor/")
}

/// Parse a source map and return the target's OWN original source files
/// (vendored / runtime sources filtered out), only those that ship content.
pub fn parse(map_text: &str) -> Result<Vec<SourceFile>> {
    let raw: RawMap =
        serde_json::from_str(map_text).map_err(|e| anyhow!("source map did not parse: {e}"))?;
    let mut out = Vec::new();
    for (i, src) in raw.sources.iter().enumerate() {
        if is_vendor(src) {
            continue;
        }
        if let Some(Some(content)) = raw.sources_content.get(i) {
            if !content.trim().is_empty() {
                out.push(SourceFile {
                    path: clean_path(src),
                    content: content.clone(),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_comment_ref() {
        let js = "console.log(1)\n//# sourceMappingURL=app.4f3a.js.map\n";
        assert_eq!(find_ref(js).as_deref(), Some("app.4f3a.js.map"));
    }

    #[test]
    fn parses_and_filters_vendor() {
        let map = r#"{"version":3,"sources":["webpack://app/./src/api/admin.js","webpack://app/./node_modules/react/index.js"],"sourcesContent":["fetch('/api/v2/admin')","module.exports=React"]}"#;
        let files = parse(map).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/api/admin.js");
        assert!(files[0].content.contains("/api/v2/admin"));
    }

    #[test]
    fn decodes_base64_data_uri() {
        // {"version":3,"sources":["a.js"],"sourcesContent":["x"]}
        let b64 = base64::engine::general_purpose::STANDARD
            .encode(r#"{"version":3,"sources":["a.js"],"sourcesContent":["x"]}"#);
        let uri = format!("data:application/json;base64,{b64}");
        let json = decode_data_uri(&uri).unwrap();
        let files = parse(&json).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "a.js");
    }
}
