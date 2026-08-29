use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/shader.wgsl");

    let source = fs::read_to_string("src/shader.wgsl").expect("failed to read Ink shader");
    let module = naga::front::wgsl::parse_str(&source).expect("failed to parse Ink shader");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("failed to validate Ink shader");
    let mut options = naga::back::spv::Options::default();
    options
        .flags
        .remove(naga::back::spv::WriterFlags::ADJUST_COORDINATE_SPACE);
    let words = naga::back::spv::write_vec(&module, &info, &options, None)
        .expect("failed to compile Ink shader to SPIR-V");
    let bytes: Vec<_> = words.into_iter().flat_map(u32::to_le_bytes).collect();
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unavailable"));
    fs::write(output.join("ink.spv"), bytes).expect("failed to write Ink SPIR-V");
}
