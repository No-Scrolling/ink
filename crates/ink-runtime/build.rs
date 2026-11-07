use std::{env, fs, path::PathBuf};

// Share the CLI's compiler so built-ins use the same format and debug information.
#[path = "src/bytecode/compiler.rs"]
mod compiler;

fn main() -> anyhow::Result<()> {
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    for name in ["encoding", "host", "events", "data_views"] {
        let path = format!("src/{name}.js");
        println!("cargo:rerun-if-changed={path}");
        let source = fs::read_to_string(path)?;
        let bytecode = compiler::compile(&source, &format!("ink/{name}.js"))?;
        fs::write(output.join(format!("{name}.jsbc")), bytecode)?;
    }
    println!("cargo:rerun-if-changed=src/bytecode/compiler.rs");
    Ok(())
}
