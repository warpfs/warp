fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        println!("cargo::rustc-link-lib=framework=CoreFoundation");
        println!("cargo::rustc-link-lib=framework=Security");
    }
}
