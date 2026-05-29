fn main() -> anyhow::Result<()> {
    println!("{}: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("{:?}", pylon_shimload::version()?);
    Ok(())
}
