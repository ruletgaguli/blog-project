fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../blog-server/proto/blog.proto");
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_with_config(
            {
                let mut config = tonic_prost_build::Config::new();
                config.protoc_executable(protoc);
                config
            },
            &["../blog-server/proto/blog.proto"],
            &["../blog-server/proto"],
        )?;
    Ok(())
}
