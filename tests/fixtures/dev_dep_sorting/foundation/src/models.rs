use crate::common;

pub struct Item {
    pub name: String,
}

impl Item {
    pub fn new(name: &str) -> Self {
        let _tag = common::shared_util();
        Self {
            name: name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use test_helper::Fixture;

    #[test]
    fn test_item_with_fixture() {
        let fix = Fixture::new("model_test");
        assert_eq!(fix.name, "model_test");
    }
}
