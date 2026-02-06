fn main() {
    #[cfg(feature = "a")]
    {
        use my_lib::config::Config;
        use my_lib::engine::Engine;
        let _engine = Engine;
        let _config = Config { verbose: true };
        my_lib::init();
    }
}
