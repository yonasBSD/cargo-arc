use crate::common;
use crate::models::Item;
use crate::service;

pub fn make_test_item() -> Item {
    let _tag = common::shared_util();
    service::process("test")
}
