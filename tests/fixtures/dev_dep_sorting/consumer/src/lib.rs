pub fn consumer_logic() -> u32 {
    42
}

#[cfg(test)]
mod tests {
    use foundation::test_support;
    use test_helper::Fixture;

    #[test]
    fn test_with_foundation() {
        let item = test_support::make_test_item();
        assert_eq!(item.name, "test");
    }

    #[test]
    fn test_with_helper() {
        let fix = Fixture::new("consumer").with_value(42);
        assert_eq!(fix.value, 42);
    }
}
