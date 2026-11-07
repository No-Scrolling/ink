fn main() {
    for text in [
        "Count: 0",
        "office ffi",
        "AVATAR",
        "1234567890",
        "مرحبا بالعالم",
        "🙂",
        "one\ttwo",
        "e\u{301}",
    ] {
        println!("metric\t{text}\t{}", ink_core::text_width(text, 46.0));
    }
}
