use protoc_bin_vendored::protoc_bin_path;
use std::path::PathBuf;

mod build_identity;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proto_dir = manifest_dir.join("proto");
    let proto_file = proto_dir.join("blit.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());
    let build_sha = build_identity::git_build_suffix(&manifest_dir)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    println!("cargo:rustc-env=BLIT_GIT_SHA={build_sha}");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto_file.as_path()], &[proto_dir.as_path()])?;
    Ok(())
}
