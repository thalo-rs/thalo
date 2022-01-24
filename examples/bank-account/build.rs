fn main() -> Result<(), Box<dyn std::error::Error>> {
    esdl::configure()
        .add_schema_file("./bank-account.esdl")?
        .compile()?;

    Ok(())
}
