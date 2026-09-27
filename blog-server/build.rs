fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/blog.proto");
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_with_config(
            {
                let mut config = tonic_prost_build::Config::new();
                config.protoc_executable(protoc);
                config
            },
            &["proto/blog.proto"],
            &["proto"],
        )?;
    Ok(())
}
