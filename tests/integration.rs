use cargo_arc::{Args, run};
use std::path::PathBuf;

#[test]
fn test_self_analysis() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let args = Args {
        output: Some(temp.path().to_path_buf()),
        manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    };

    let result = run(args);
    assert!(result.is_ok(), "run() should succeed: {:?}", result);

    let svg = std::fs::read_to_string(temp.path()).unwrap();

    // Valid SVG structure
    assert!(svg.contains("<?xml"), "should have XML declaration");
    assert!(svg.contains("<svg"), "should have svg element");
    assert!(svg.contains("</svg>"), "should close svg element");

    // All cargo-arc modules visible
    assert!(svg.contains("analyze"), "should show analyze module");
    assert!(svg.contains("graph"), "should show graph module");
    assert!(svg.contains("layout"), "should show layout module");
    assert!(svg.contains("render"), "should show render module");
}
