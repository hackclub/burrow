fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile_protos(
        &["../proto/burrow.proto", "../proto/burrowweb.proto"],
        &["../proto", "../proto"],
    )?;
    Ok(())
}
