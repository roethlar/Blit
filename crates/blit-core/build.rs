use protoc_bin_vendored::protoc_bin_path;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proto_dir = manifest_dir.join("..").join("..").join("proto");
    let proto_file = proto_dir.join("blit.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&[proto_file.as_path()], &[proto_dir.as_path()])?;
    Ok(())
}
