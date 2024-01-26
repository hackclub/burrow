use anyhow::Result;

fn main() -> Result<()> {
    compile_gresources()?;

    Ok(())
}

fn compile_gresources() -> Result<()> {
    glib_build_tools::compile_resources(
        &["data"],
        "data/resources.gresource.xml",
        "compiled.gresource",
    );
    Ok(())
}
