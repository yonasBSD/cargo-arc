use crate::common;
use crate::models::Item;

// Cycle: service uses handler::handle_request indirectly via re-export pattern
// In real code this could be a callback, trait impl, or cross-module reference
use crate::handler;

pub fn process(name: &str) -> Item {
    let _tag = common::shared_util();
    Item::new(name)
}

pub fn process_with_handler() -> String {
    handler::handle_request()
}
