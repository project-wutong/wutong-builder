// SPDX-License-Identifier: AGPL-3.0-only
//! Wutong Builder engine.
//! Service spec model (WTP §12) and `spec_digest` computation.

use serde_json::Value;

/// Sections every service spec must declare.
pub const SECTIONS: [&str; 8] = [
    "sources",
    "types",
    "queries",
    "views",
    "actions",
    "policies",
    "runtime",
    "distribution",
];

/// Validate a spec: all sections present, no embedded secrets; returns its JCS `spec_digest`.
pub fn spec_digest(spec_json: &str) -> Result<String, String> {
    let v = wutong_core::wire::strict::parse(spec_json.as_bytes()).map_err(|e| e.to_string())?;
    let obj = v.as_object().ok_or("spec must be an object")?;
    for s in SECTIONS {
        if !obj.contains_key(s) {
            return Err(format!("missing section `{s}`"));
        }
    }
    fn scan(v: &Value) -> bool {
        match v {
            Value::String(s) => {
                let l = s.to_lowercase();
                l.contains("-----begin")
                    || l.starts_with("bearer ")
                    || l.starts_with("ghp_")
                    || l.starts_with("sk-")
            }
            Value::Array(a) => a.iter().any(scan),
            Value::Object(o) => o.iter().any(|(k, x)| {
                matches!(k.as_str(), "token" | "secret" | "password" | "private_key") || scan(x)
            }),
            _ => false,
        }
    }
    if scan(&v) {
        return Err("service specs must not embed tokens or private keys".into());
    }
    Ok(wutong_core::wire::canonical_digest(&v)
        .map_err(|e| e.to_string())?
        .to_string())
}

/// One-line description used by the GUI.
pub fn describe(bytes: Vec<u8>) -> String {
    match std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())
        .and_then(spec_digest)
    {
        Ok(d) => format!("valid service spec · spec_digest {d}"),
        Err(e) => format!("invalid service spec: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(extra: &str) -> String {
        format!(
            r#"{{"sources":[],"types":{{}},"queries":{{}},"views":{{}},"actions":{{}},"policies":{{}},"runtime":{{"kind":"static_site"}},"distribution":{{}}{extra}}}"#
        )
    }

    #[test]
    fn digest_is_order_independent_and_secrets_rejected() {
        let a = spec_digest(&spec("")).unwrap();
        let b = spec_digest(r#"{"distribution":{},"runtime":{"kind":"static_site"},"policies":{},"actions":{},"views":{},"queries":{},"types":{},"sources":[]}"#).unwrap();
        assert_eq!(a, b);
        assert!(spec_digest(&spec(r#","token":"abc""#)).is_err());
        assert!(spec_digest(r#"{"sources":[]}"#).is_err());
    }
}
