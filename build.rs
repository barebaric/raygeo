fn main() {
    if std::env::var("CARGO_FEATURE_PYTHON").is_ok() {
        println!("cargo:rustc-crate-type=cdylib");
    }
}
