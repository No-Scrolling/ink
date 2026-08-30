#[cfg(target_os = "android")]
mod android;
#[cfg(all(target_os = "android", feature = "audio"))]
mod audio;
#[cfg(target_os = "android")]
mod generated_app {
    include!(env!("INK_APP_RS"));
}
