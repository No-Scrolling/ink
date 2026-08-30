#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod generated_app {
    include!(env!("INK_APP_RS"));
}
