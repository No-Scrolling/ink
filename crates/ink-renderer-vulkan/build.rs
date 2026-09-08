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
    options.flags.remove(
        naga::back::spv::WriterFlags::ADJUST_COORDINATE_SPACE | naga::back::spv::WriterFlags::DEBUG,
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unavailable"));
    for (entry_point, shader_stage) in [
        ("quad_vertex", naga::ShaderStage::Vertex),
        ("quad_fragment", naga::ShaderStage::Fragment),
        ("text_vertex", naga::ShaderStage::Vertex),
        ("text_fragment", naga::ShaderStage::Fragment),
        ("image_fragment", naga::ShaderStage::Fragment),
    ] {
        let pipeline = naga::back::spv::PipelineOptions {
            shader_stage,
            entry_point: entry_point.to_owned(),
        };
        let words = naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline))
            .expect("failed to compile Ink shader to SPIR-V");
        let bytes: Vec<_> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        fs::write(output.join(format!("{entry_point}.spv")), bytes)
            .expect("failed to write Ink SPIR-V");
    }
}
