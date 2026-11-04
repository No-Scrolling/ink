-keepclasseswithmembernames class com.vandam.ink.InkNative {
    native <methods>;
}

# Called from Rust through JNI when JavaScript has queued work.
-keepclassmembers class com.vandam.ink.MainActivity {
    public void onJavaScriptReady();
    public java.lang.String loadWebRuntime();
}

-keep interface com.vandam.ink.InkJavaScriptHost { *; }
-keepclassmembers class * implements com.vandam.ink.InkJavaScriptHost {
    public void onJavaScriptReady();
}

-keepclasseswithmembernames class com.vandam.ink.InkDirectPhotoCamera {
    native <methods>;
}

# Constructed and called by Rust database workers.
-keep class com.vandam.ink.InkDatabases { public *; }
-keep class com.vandam.ink.InkDatabaseConnection { public *; }
