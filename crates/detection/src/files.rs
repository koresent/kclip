use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedFileList {
    pub paths: Vec<PathBuf>,
    pub count: usize,
}

pub fn detect_file_list(uri_list_text: &str) -> Option<DetectedFileList> {
    let mut paths = Vec::new();

    for line in uri_list_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "copy" || trimmed == "cut" {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("file://") {
            let decoded = urlencoding_decode(rest);
            let path = PathBuf::from(decoded);
            paths.push(path);
        }
    }

    if paths.is_empty() {
        None
    } else {
        let count = paths.len();
        Some(DetectedFileList { paths, count })
    }
}

fn urlencoding_decode(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.bytes();

    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2)
                && let Ok(num) =
                    u8::from_str_radix(std::str::from_utf8(&[c1, c2]).unwrap_or(""), 16)
            {
                bytes.push(num);
                continue;
            }
        }
        bytes.push(b);
    }

    String::from_utf8_lossy(&bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uri_list() {
        let text = "file:///home/user/document.pdf\r\nfile:///home/user/photo%201.png\r\n";
        let res = detect_file_list(text).unwrap();
        assert_eq!(res.count, 2);
        assert_eq!(res.paths[0], PathBuf::from("/home/user/document.pdf"));
        assert_eq!(res.paths[1], PathBuf::from("/home/user/photo 1.png"));
    }
}
