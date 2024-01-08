use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("event_indexer_descriptor.bin"))
        .compile(&["proto/event_indexer.proto"], &["proto"])?;

    // tonic_build::compile_protos("proto/event_indexer.proto")?;

    Ok(())
}
