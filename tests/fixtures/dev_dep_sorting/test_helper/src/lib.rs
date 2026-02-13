/// Test fixture builder for integration tests.
pub struct Fixture {
    pub name: String,
    pub value: u32,
}

impl Fixture {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: shared_lib::shared_value(),
        }
    }

    pub fn with_value(mut self, value: u32) -> Self {
        self.value = value;
        self
    }
}

pub fn assert_approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
