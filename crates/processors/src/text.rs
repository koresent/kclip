pub fn sanitize_text(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut prev_was_cr = false;

    for c in input.chars() {
        if c == '\r' {
            cleaned.push('\n');
            prev_was_cr = true;
        } else if c == '\n' {
            if !prev_was_cr {
                cleaned.push('\n');
            }
            prev_was_cr = false;
        } else if c == '\u{FEFF}' || c == '\u{200B}' {
            // Drop zero-width spaces / BOM artifacts
            continue;
        } else {
            cleaned.push(c);
            prev_was_cr = false;
        }
    }

    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crlf_normalization() {
        let windows = "line 1\r\nline 2\r\nline 3";
        assert_eq!(sanitize_text(windows), "line 1\nline 2\nline 3");

        let cr_only = "line 1\rline 2";
        assert_eq!(sanitize_text(cr_only), "line 1\nline 2");
    }
}
