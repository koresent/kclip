use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedUrl {
    pub raw: String,
    pub scheme: String,
    pub host: Option<String>,
}

pub fn detect_url(input: &str) -> Option<DetectedUrl> {
    let clean: String = input.chars().filter(|c| *c != '\0').collect();
    let s = clean.trim();
    if s.is_empty() {
        return None;
    }

    let candidate = if s.contains('\n') || s.contains(' ') {
        let first = s.lines().next().unwrap_or("").trim();
        if first.starts_with("http://") || first.starts_with("https://") {
            first.split_whitespace().next().unwrap_or("")
        } else {
            return None;
        }
    } else {
        s
    };

    if candidate.is_empty() {
        return None;
    }

    if let Ok(parsed) = Url::parse(candidate) {
        let scheme = parsed.scheme().to_ascii_lowercase();
        if scheme == "http" || scheme == "https" || scheme == "ftp" || scheme == "gemini" {
            let host = parsed.host_str().map(|h| h.to_string());
            return Some(DetectedUrl {
                raw: candidate.to_string(),
                scheme,
                host,
            });
        }
    }

    // Bare domain check (e.g. chatgpt.com, github.com/path, www.google.com)
    if !candidate.contains("://") && (candidate.starts_with("www.") || candidate.contains('.')) {
        let with_https = format!("https://{}", candidate);
        if let Ok(parsed) = Url::parse(&with_https) {
            if let Some(host) = parsed.host_str() {
                if host.contains('.') && !host.ends_with('.') && !candidate.contains(' ') {
                    return Some(DetectedUrl {
                        raw: with_https,
                        scheme: "https".to_string(),
                        host: Some(host.to_string()),
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        let u1 = detect_url("https://github.com/koresent/kclip").unwrap();
        assert_eq!(u1.scheme, "https");
        assert_eq!(u1.host.as_deref(), Some("github.com"));

        let u2 = detect_url("http://localhost:8080/api").unwrap();
        assert_eq!(u2.host.as_deref(), Some("localhost"));

        let u3 = detect_url("chatgpt.com").unwrap();
        assert_eq!(u3.scheme, "https");
        assert_eq!(u3.host.as_deref(), Some("chatgpt.com"));

        let u4 = detect_url("https://chatgpt.com\n").unwrap();
        assert_eq!(u4.host.as_deref(), Some("chatgpt.com"));
    }

    #[test]
    fn test_invalid_urls() {
        assert!(detect_url("hello world").is_none());
        assert!(detect_url("not a url\njust text").is_none());
        assert!(detect_url("javascript:alert(1)").is_none());
    }
}
