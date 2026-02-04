use cargo_arc::{Args, run};
use std::path::PathBuf;

#[test]
fn test_multi_crate_fixture() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_crate/Cargo.toml");

    let temp = tempfile::NamedTempFile::new().unwrap();
    let args = Args {
        output: Some(temp.path().to_path_buf()),
        manifest_path: fixture_path,
        features: vec![],
        all_features: false,
        no_default_features: false,
        cfg: vec![],
        debug: false,
        volatility: false,
        no_volatility: false,
        volatility_months: 6,
        volatility_low: 2,
        volatility_high: 10,
        #[cfg(feature = "hir")]
        hir: false,
    };

    let result = run(args);
    assert!(result.is_ok(), "run() should succeed: {:?}", result);

    let svg = std::fs::read_to_string(temp.path()).unwrap();

    // Valid SVG structure
    assert!(svg.contains("<svg"), "should have svg element");

    // Both crates visible
    assert!(svg.contains("crate_a"), "should show crate_a");
    assert!(svg.contains("crate_b"), "should show crate_b");

    // Modules visible
    assert!(svg.contains("alpha"), "should show alpha module");
    assert!(svg.contains("beta"), "should show beta module");
    assert!(svg.contains("gamma"), "should show gamma module");
}

#[test]
fn test_self_analysis() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let args = Args {
        output: Some(temp.path().to_path_buf()),
        manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        features: vec![],
        all_features: false,
        no_default_features: false,
        cfg: vec![],
        debug: false,
        volatility: false,
        no_volatility: false,
        volatility_months: 6,
        volatility_low: 2,
        volatility_high: 10,
        #[cfg(feature = "hir")]
        hir: false,
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

#[test]
fn test_cfg_test_excluded_by_default() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_crate/Cargo.toml");

    let temp = tempfile::NamedTempFile::new().unwrap();
    let args = Args {
        output: Some(temp.path().to_path_buf()),
        manifest_path: fixture_path,
        features: vec![],
        all_features: false,
        no_default_features: false,
        cfg: vec![], // No --cfg test flag
        debug: false,
        volatility: false,
        no_volatility: false,
        volatility_months: 6,
        volatility_low: 2,
        volatility_high: 10,
        #[cfg(feature = "hir")]
        hir: false,
    };

    let result = run(args);
    assert!(result.is_ok(), "run() should succeed: {:?}", result);

    let svg = std::fs::read_to_string(temp.path()).unwrap();

    // test_utils module should NOT be visible (cfg(test) is excluded by default)
    assert!(
        !svg.contains("test_utils"),
        "test_utils should be hidden by default (cfg(test) excluded)"
    );
}

#[test]
fn test_cfg_test_included_with_flag() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_crate/Cargo.toml");

    let temp = tempfile::NamedTempFile::new().unwrap();
    let args = Args {
        output: Some(temp.path().to_path_buf()),
        manifest_path: fixture_path,
        features: vec![],
        all_features: false,
        no_default_features: false,
        cfg: vec!["test".to_string()], // --cfg test flag
        debug: false,
        volatility: false,
        no_volatility: false,
        volatility_months: 6,
        volatility_low: 2,
        volatility_high: 10,
        #[cfg(feature = "hir")]
        hir: false,
    };

    let result = run(args);
    assert!(result.is_ok(), "run() should succeed: {:?}", result);

    let svg = std::fs::read_to_string(temp.path()).unwrap();

    // test_utils module SHOULD be visible when --cfg test is passed
    assert!(
        svg.contains("test_utils"),
        "test_utils should be visible with --cfg test"
    );
}

#[test]
fn test_entry_point_imports() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/entry_point/Cargo.toml");

    let temp = tempfile::NamedTempFile::new().unwrap();
    let args = Args {
        output: Some(temp.path().to_path_buf()),
        manifest_path: fixture_path,
        features: vec![],
        all_features: false,
        no_default_features: false,
        cfg: vec![],
        debug: false,
        volatility: false,
        no_volatility: false,
        volatility_months: 6,
        volatility_low: 2,
        volatility_high: 10,
        #[cfg(feature = "hir")]
        hir: false,
    };

    let result = run(args);
    assert!(result.is_ok(), "run() should succeed: {:?}", result);

    let svg = std::fs::read_to_string(temp.path()).unwrap();

    // Valid SVG structure
    assert!(svg.contains("<svg"), "should have svg element");

    // Both crates visible
    assert!(svg.contains("crate_a"), "should show crate_a");
    assert!(svg.contains("crate_b"), "should show crate_b");

    // Modules visible
    assert!(svg.contains("sub"), "should show sub module in crate_a");
    assert!(svg.contains("mod_b"), "should show mod_b module in crate_b");

    // Entry-point imports should create arcs with source locations (shown in STATIC_DATA).
    // Helper is imported from crate_a's entry point in crate_b's lib.rs,
    // Exported is imported from crate_a's entry point in crate_b's mod_b.rs.
    assert!(
        svg.contains("Helper") || svg.contains("Exported"),
        "SVG should contain entry-point symbol names in STATIC_DATA usages"
    );
}
