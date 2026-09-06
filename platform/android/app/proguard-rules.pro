-keepclasseswithmembernames class com.vandam.ink.MainActivity {
    native <methods>;
}

# Called from Rust through JNI when JavaScript has queued work.
-keepclassmembers class com.vandam.ink.MainActivity {
    public void onJavaScriptReady();
    public java.lang.String loadWebRuntime();
}
