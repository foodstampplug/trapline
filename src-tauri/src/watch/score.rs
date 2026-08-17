use super::parse::{Artifact, Kind};

/// How juicy is this newly-appeared artifact? Higher = look at it first.
pub fn score(a: &Artifact) -> i64 {
    let v = a.value.to_lowercase();
    match a.kind {
        Kind::Secret => 100,
        Kind::Flag => 25,
        Kind::Endpoint | Kind::Route => {
            let mut s = 30;
            const KW: [(&str, i64); 20] = [
                ("admin", 85),
                ("internal", 85),
                ("debug", 75),
                ("root", 70),
                ("superuser", 85),
                ("graphql", 60),
                ("/v2", 55),
                ("/v3", 55),
                ("token", 70),
                ("auth", 65),
                ("apikey", 70),
                ("password", 75),
                ("secret", 75),
                ("upload", 55),
                ("webhook", 55),
                ("ssrf", 60),
                ("redirect", 50),
                ("role", 60),
                ("account", 45),
                ("config", 50),
            ];
            for (kw, pts) in KW {
                if v.contains(kw) {
                    s = s.max(pts);
                }
            }
            s
        }
    }
}

pub fn severity(score: i64) -> &'static str {
    if score >= 90 {
        "Critical"
    } else if score >= 65 {
        "High"
    } else if score >= 45 {
        "Medium"
    } else {
        "Low"
    }
}

/// Discord embed color (decimal) for a severity band.
pub fn color(score: i64) -> i64 {
    match severity(score) {
        "Critical" => 0xFF3C60,
        "High" => 0xFEBC2E,
        "Medium" => 0x63B3ED,
        _ => 0x1DB954,
    }
}
