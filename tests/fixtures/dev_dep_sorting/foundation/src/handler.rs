use crate::common;
use crate::service;

pub fn handle_request() -> String {
    let item = service::process("request");
    format!("{}: {}", common::shared_util(), item.name)
}
