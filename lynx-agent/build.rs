fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .out_dir("src/proto")
        .protoc_arg("-I=../lynx-proto")
        .compile_protos(
            &["monitor.proto"],
            &["."],
        )?;
    Ok(())
}
