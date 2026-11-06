use url::Url;

const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "fbclid",
    "gclid",
    "dclid",
    "msclkid",
    "mc_cid",
    "mc_eid",
    "igshid",
    "_ga",
    "_gl",
];

pub fn clean_url(raw_url: &str) -> String {
    let mut parsed = match Url::parse(raw_url) {
        Ok(u) => u,
        Err(_) => return raw_url.to_string(),
    };

    let filtered: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| {
            let k = key.to_ascii_lowercase();
            !TRACKING_PARAMS.contains(&k.as_str()) && !k.starts_with("utm_")
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if filtered.is_empty() {
        parsed.set_query(None);
    } else {
        parsed.query_pairs_mut().clear().extend_pairs(filtered);
    }

    parsed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_tracking_url() {
        let dirty =
            "https://example.com/article?id=123&utm_source=twitter&utm_medium=social&fbclid=XYZ123";
        let cleaned = clean_url(dirty);
        assert_eq!(cleaned, "https://example.com/article?id=123");

        let clean = "https://example.com/article?id=123";
        assert_eq!(clean_url(clean), clean);

        let dirty_only = "https://example.com/?utm_campaign=winter_sale";
        assert_eq!(clean_url(dirty_only), "https://example.com/");
    }
}
