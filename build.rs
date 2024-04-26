fn main() {
    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => {
            println!("cargo::rustc-link-lib=framework=CoreFoundation");
            println!("cargo::rustc-link-lib=framework=Security");
            println!("cargo::rerun-if-changed=src/key/store/default.m");

            cc::Build::new()
                .file("src/key/store/default.m")
                .compile("warpffi")
        }
        _ => {}
    }
}
