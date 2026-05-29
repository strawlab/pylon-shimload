use pylon_shimload::HasProperties;

fn main() -> anyhow::Result<()> {
    for device in pylon_shimload::enumerate_devices()? {
        println!(
            "Device {} {} -------------",
            device.property_value("VendorName")?,
            device.property_value("SerialNumber")?
        );
        for name in device.property_names()? {
            let value = device.property_value(&name)?;
            println!("  {}: {}", name, value);
        }
    }
    Ok(())
}
