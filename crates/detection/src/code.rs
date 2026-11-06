#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedCode {
    pub language_hint: Option<String>,
}

pub fn detect_code(text: &str) -> Option<DetectedCode> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() < 8 {
        return None;
    }

    // 1. Shebang
    if trimmed.starts_with("#!") {
        let first_line = trimmed.lines().next().unwrap_or("");
        let hint = if first_line.contains("python") {
            "python"
        } else if first_line.contains("bash") || first_line.contains("sh") {
            "bash"
        } else if first_line.contains("node") {
            "javascript"
        } else {
            "shell"
        };
        return Some(DetectedCode {
            language_hint: Some(hint.to_string()),
        });
    }

    // 2. Strong keyword / signature matching
    let mut score = 0;
    let mut detected_lang: Option<&'static str> = None;

    if trimmed.contains("pub fn ")
        || trimmed.contains("impl ")
        || (trimmed.contains("fn ") && (trimmed.contains("->") || trimmed.contains('(')))
    {
        score += 3;
        detected_lang = Some("rust");
    } else if trimmed.contains("def ") && trimmed.contains(":") {
        score += 3;
        detected_lang = Some("python");
    } else if trimmed.contains("import React")
        || trimmed.contains("const ") && trimmed.contains("=>")
    {
        score += 3;
        detected_lang = Some("typescript");
    } else if trimmed.contains("#include <") {
        score += 3;
        detected_lang = Some("cpp");
    } else if trimmed.contains("public class ") || trimmed.contains("public static void main") {
        score += 3;
        detected_lang = Some("java");
    } else if trimmed.contains("package main") && trimmed.contains("func ") {
        score += 3;
        detected_lang = Some("go");
    } else if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']')))
        && trimmed.contains(':')
        && (trimmed.contains('"') || trimmed.contains(','))
    {
        return Some(DetectedCode {
            language_hint: Some("json".to_string()),
        });
    }

    // 3. Structural markers: indentation, braces, semicolons
    let brace_count = trimmed
        .chars()
        .filter(|&c| c == '{' || c == '}' || c == ';')
        .count();
    let line_count = trimmed.lines().count();

    if brace_count >= 2 && line_count >= 2 {
        score += 2;
    }

    if score >= 3 {
        Some(DetectedCode {
            language_hint: detected_lang.map(|s| s.to_string()),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_detection() {
        let rust_code = "pub fn calculate_sum(a: i32, b: i32) -> i32 {\n    a + b\n}";
        let res = detect_code(rust_code).unwrap();
        assert_eq!(res.language_hint.as_deref(), Some("rust"));

        let py_code = "def process_data(items):\n    return [x * 2 for x in items]\n";
        let res_py = detect_code(py_code).unwrap();
        assert_eq!(res_py.language_hint.as_deref(), Some("python"));

        let prose = "This is a regular plain text sentence that has no programming code inside.";
        assert!(detect_code(prose).is_none());
    }
}
