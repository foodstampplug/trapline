use serde::{Deserialize, Serialize};

const BASE: &str = "https://api.shodan.io";

pub fn host_url(key: &str, ip: &str) -> String {
    format!("{BASE}/shodan/host/{ip}?key={key}")
}
pub fn domain_url(key: &str, domain: &str) -> String {
    format!("{BASE}/dns/domain/{domain}?key={key}")
}
pub fn search_url(key: &str, query: &str) -> String {
    format!("{BASE}/shodan/host/search?key={key}&query={}", urlencoding(query))
}

/// Minimal query-string escaper (avoid pulling a new dep): space + a few reserved chars.
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".to_string(),
            _ => format!("%{:02X}", c as u32 & 0xFF),
        })
        .collect()
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanService {
    pub port: u16,
    pub product: String,
    pub version: String,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanHost {
    pub ip: String,
    pub org: String,
    pub hostnames: Vec<String>,
    pub ports: Vec<u16>,
    pub services: Vec<ShodanService>,
    pub cves: Vec<String>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanRecord {
    pub kind: String,
    pub value: String,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanDomain {
    pub domain: String,
    pub subdomains: Vec<String>,
    pub records: Vec<ShodanRecord>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanMatch {
    pub ip: String,
    pub port: u16,
    pub org: String,
    pub product: String,
    pub cves: Vec<String>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanSearch {
    pub total: u64,
    pub matches: Vec<ShodanMatch>,
}

#[derive(Deserialize, Default)]
struct RawHost {
    #[serde(default)]
    ip_str: String,
    #[serde(default)]
    org: String,
    #[serde(default)]
    hostnames: Vec<String>,
    #[serde(default)]
    ports: Vec<u16>,
    #[serde(default)]
    vulns: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default)]
    data: Vec<RawService>,
}

#[derive(Deserialize, Default)]
struct RawService {
    #[serde(default)]
    port: u16,
    #[serde(default)]
    product: String,
    #[serde(default)]
    version: String,
}

pub fn parse_host(body: &str) -> ShodanHost {
    let raw: RawHost = serde_json::from_str(body).unwrap_or_default();
    let mut cves: Vec<String> = raw.vulns.keys().cloned().collect();
    cves.sort();
    ShodanHost {
        ip: raw.ip_str,
        org: raw.org,
        hostnames: raw.hostnames,
        ports: raw.ports,
        services: raw
            .data
            .into_iter()
            .map(|d| ShodanService { port: d.port, product: d.product, version: d.version })
            .collect(),
        cves,
    }
}

#[derive(Deserialize, Default)]
struct RawDomain {
    #[serde(default)]
    domain: String,
    #[serde(default)]
    subdomains: Vec<String>,
    #[serde(default)]
    data: Vec<RawDnsEntry>,
}

#[derive(Deserialize, Default)]
struct RawDnsEntry {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    value: String,
}

pub fn parse_domain(body: &str) -> ShodanDomain {
    let raw: RawDomain = serde_json::from_str(body).unwrap_or_default();
    ShodanDomain {
        domain: raw.domain,
        subdomains: raw.subdomains,
        records: raw
            .data
            .into_iter()
            .map(|e| ShodanRecord { kind: e.kind, value: e.value })
            .collect(),
    }
}

#[derive(Deserialize, Default)]
struct RawSearch {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    matches: Vec<RawMatch>,
}

#[derive(Deserialize, Default)]
struct RawMatch {
    #[serde(default)]
    ip_str: String,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    org: String,
    #[serde(default)]
    product: String,
    #[serde(default)]
    vulns: std::collections::HashMap<String, serde_json::Value>,
}

pub fn parse_search(body: &str) -> ShodanSearch {
    let raw: RawSearch = serde_json::from_str(body).unwrap_or_default();
    ShodanSearch {
        total: raw.total,
        matches: raw
            .matches
            .into_iter()
            .map(|m| {
                let mut cves: Vec<String> = m.vulns.keys().cloned().collect();
                cves.sort();
                ShodanMatch { ip: m.ip_str, port: m.port, org: m.org, product: m.product, cves }
            })
            .collect(),
    }
}

/// GET a Shodan URL, returning the body. Scrubs the API key from any error
/// string — reqwest's Error Display embeds the request URL, which contains
/// `?key=<KEY>`, so a raw `e.to_string()` would leak the key to the UI toast.
async fn fetch(key: &str, url: &str) -> Result<String, String> {
    let resp = super::client().get(url).send().await.map_err(|e| scrub(&e.to_string(), key))?;
    resp.text().await.map_err(|e| scrub(&e.to_string(), key))
}

/// Replace every occurrence of the key with `***` (belt-and-suspenders: catches
/// the key wherever reqwest embeds it, not just in the `?key=` position).
fn scrub(msg: &str, key: &str) -> String {
    if key.is_empty() { msg.to_string() } else { msg.replace(key, "***") }
}

pub async fn host(key: &str, ip: &str) -> Result<ShodanHost, String> {
    if key.trim().is_empty() {
        return Err("Shodan API key not set — add it in Settings".into());
    }
    let body = fetch(key, &host_url(key, ip)).await?;
    Ok(parse_host(&body))
}

pub async fn domain(key: &str, domain: &str) -> Result<ShodanDomain, String> {
    if key.trim().is_empty() {
        return Err("Shodan API key not set — add it in Settings".into());
    }
    let body = fetch(key, &domain_url(key, domain)).await?;
    Ok(parse_domain(&body))
}

pub async fn search(key: &str, query: &str) -> Result<ShodanSearch, String> {
    if key.trim().is_empty() {
        return Err("Shodan API key not set — add it in Settings".into());
    }
    let body = fetch(key, &search_url(key, query)).await?;
    Ok(parse_search(&body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_place_key_and_args() {
        assert_eq!(host_url("K", "1.2.3.4"), "https://api.shodan.io/shodan/host/1.2.3.4?key=K");
        assert_eq!(domain_url("K", "acme.com"), "https://api.shodan.io/dns/domain/acme.com?key=K");
        assert!(search_url("K", "apache port:443").starts_with("https://api.shodan.io/shodan/host/search?key=K&query="));
        assert!(search_url("K", "a b").contains("query=a%20b") || search_url("K", "a b").contains("query=a+b"));
    }

    #[test]
    fn parse_host_pulls_ports_services_cves() {
        let sample = r#"{
          "ip_str":"1.2.3.4","org":"Acme","hostnames":["a.acme.com"],
          "ports":[80,443],
          "vulns":{"CVE-2021-1234":{},"CVE-2020-5678":{}},
          "data":[{"port":443,"product":"nginx","version":"1.20"},{"port":80,"product":"","version":""}]
        }"#;
        let h = parse_host(sample);
        assert_eq!(h.ip, "1.2.3.4");
        assert_eq!(h.org, "Acme");
        assert_eq!(h.ports, vec![80, 443]);
        assert!(h.cves.contains(&"CVE-2021-1234".to_string()));
        assert!(h.services.iter().any(|s| s.port == 443 && s.product == "nginx"));
    }

    #[test]
    fn parse_host_tolerates_missing_fields() {
        // Unknown/empty shape → empty-but-Ok, never a panic.
        let h = parse_host("{}");
        assert_eq!(h.ip, "");
        assert!(h.ports.is_empty());
        assert!(h.cves.is_empty());
    }

    #[test]
    fn parse_domain_pulls_subdomains_and_records() {
        let sample = r#"{
          "domain":"acme.com",
          "subdomains":["www","api"],
          "data":[{"subdomain":"www","type":"A","value":"1.2.3.4"},{"subdomain":"api","type":"CNAME","value":"api.lb.acme.com"}]
        }"#;
        let d = parse_domain(sample);
        assert_eq!(d.domain, "acme.com");
        assert_eq!(d.subdomains, vec!["www".to_string(), "api".to_string()]);
        assert_eq!(d.records.len(), 2);
        assert!(d.records.iter().any(|r| r.kind == "A" && r.value == "1.2.3.4"));
    }

    #[test]
    fn parse_domain_tolerates_missing_fields() {
        let d = parse_domain("{}");
        assert_eq!(d.domain, "");
        assert!(d.subdomains.is_empty());
        assert!(d.records.is_empty());
    }

    #[test]
    fn parse_search_pulls_total_and_matches() {
        let sample = r#"{
          "total": 2,
          "matches": [
            {"ip_str":"1.2.3.4","port":443,"org":"Acme","product":"nginx","vulns":{"CVE-2021-1234":{}}},
            {"ip_str":"5.6.7.8","port":80,"org":"","product":"","vulns":{}}
          ]
        }"#;
        let s = parse_search(sample);
        assert_eq!(s.total, 2);
        assert_eq!(s.matches.len(), 2);
        let m = &s.matches[0];
        assert_eq!(m.ip, "1.2.3.4");
        assert_eq!(m.port, 443);
        assert_eq!(m.org, "Acme");
        assert_eq!(m.product, "nginx");
        assert!(m.cves.contains(&"CVE-2021-1234".to_string()));
    }

    #[test]
    fn parse_search_tolerates_missing_fields() {
        let s = parse_search("{}");
        assert_eq!(s.total, 0);
        assert!(s.matches.is_empty());
    }

    #[test]
    fn scrub_removes_key_from_error_string() {
        let key = "SHODAN_SECRET_KEY_123";
        let leaky = format!("error sending request for url (https://api.shodan.io/shodan/host/1.2.3.4?key={key})");
        let cleaned = scrub(&leaky, key);
        assert!(!cleaned.contains(key), "key must be scrubbed from error, got: {cleaned}");
        assert!(cleaned.contains("***"));
        // empty key must not turn the whole string into ***
        assert_eq!(scrub("some error", ""), "some error");
    }

    #[test]
    fn async_fns_reject_empty_key() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(host("", "1.2.3.4")).unwrap_err();
        assert!(err.contains("Shodan API key not set"));
        let err = rt.block_on(domain("  ", "acme.com")).unwrap_err();
        assert!(err.contains("Shodan API key not set"));
        let err = rt.block_on(search("\t", "apache")).unwrap_err();
        assert!(err.contains("Shodan API key not set"));
    }
}
