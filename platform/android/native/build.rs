fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android")
        && std::env::var_os("CARGO_FEATURE_NETWORK").is_some()
    {
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
        println!("cargo:rustc-link-arg=-ljnigraphics");
        println!("cargo:rustc-link-arg=-Wl,--as-needed");
    }
}
