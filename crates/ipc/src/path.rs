use std::path::PathBuf;

pub fn resolve_socket_path(override_path: Option<&PathBuf>) -> PathBuf {
    if let Some(p) = override_path {
        return p.clone();
    }

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime_dir).join("kclip.sock");
        return p;
    }

    let uid = rustix::process::getuid().as_raw();
    PathBuf::from(format!("/tmp/kclip-{}.sock", uid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_override() {
        let custom = PathBuf::from("/tmp/custom.sock");
        assert_eq!(resolve_socket_path(Some(&custom)), custom);
    }
}
