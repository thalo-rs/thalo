use esdl::codegen::{rust::RustCompiler, Compiler};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Compiler::new(RustCompiler)
        .add_schema_file("./bank-account.esdl")?
        .compile()?;

    Ok(())
}
