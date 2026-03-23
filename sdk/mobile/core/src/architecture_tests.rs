#[test]
fn core_should_not_use_uniffi() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src_path = std::path::Path::new(&manifest_dir).join("src");

    let mut violations = Vec::new();
    scan_directory(&src_path, &mut violations);

    // Exclude this test file itself.
    violations.retain(|v| !v.contains("architecture_tests.rs"));

    if !violations.is_empty() {
        panic!(
            "\nFound {} UniFFI usage(s) in core crate!\n\n{}\n\n\
             The core crate must remain pure Rust with no UniFFI dependency.\n\
             UniFFI annotations belong in the ffi crate only.\n",
            violations.len(),
            violations.join("\n")
        );
    }
}

fn scan_directory(dir: &std::path::Path, violations: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            scan_directory(&path, violations);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (line_num, line) in content.lines().enumerate() {
                    if line.contains("uniffi") {
                        violations.push(format!(
                            "  {}:{} - {}",
                            path.display(),
                            line_num + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
    }
}
