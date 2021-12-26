fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(&["proto/bank_account.proto"], &["proto"])?;
    Ok(())
}
