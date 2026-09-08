fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        // RELR is supported from API 30; Ink targets API 34 and later.
        println!("cargo:rustc-link-arg=-Wl,--pack-dyn-relocs=relr");
        if std::env::var_os("CARGO_FEATURE_IMAGE").is_some() {
            println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
            println!("cargo:rustc-link-arg=-ljnigraphics");
            println!("cargo:rustc-link-arg=-Wl,--as-needed");
        }
    }
}
