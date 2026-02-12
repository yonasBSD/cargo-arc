use crate_lib::something;

#[test]
fn test_check() {
    assert_eq!(something(), "hello");
}
