fn main() {
    println!("cargo:rerun-if-env-changed=INK_APP_RS");
    if let Some(path) = std::env::var_os("INK_APP_RS") {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android")
        && std::env::var_os("CARGO_FEATURE_NETWORK").is_some()
    {
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
        println!("cargo:rustc-link-arg=-ljnigraphics");
        println!("cargo:rustc-link-arg=-Wl,--as-needed");
    }
}
