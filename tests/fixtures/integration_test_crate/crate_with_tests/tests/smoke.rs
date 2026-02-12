mod common;

use crate_with_tests::helper;
use crate_lib::something;

#[test]
fn test_smoke() {
    assert_eq!(helper(), "helper");
    assert_eq!(something(), "hello");
    common::setup();
}
