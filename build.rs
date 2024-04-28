fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        println!("cargo::rustc-link-lib=framework=CoreFoundation");
        println!("cargo::rustc-link-lib=framework=Security");
        println!("cargo::rerun-if-changed=src/key/store/default.m");

        cc::Build::new()
            .file("src/key/store/default.m")
            .compile("warpffi")
    }
}
