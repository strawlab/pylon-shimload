use pylon_shimload::HasProperties;

fn main() -> anyhow::Result<()> {
    for device in pylon_shimload::enumerate_devices()? {
        println!(
            "Device {} {} -------------",
            device.property_value("VendorName")?,
            device.property_value("SerialNumber")?
        );

        let camera = pylon_shimload::create_device(&device)?;
        camera.open()?;

        {
            let node = camera.node_map()?.command_node("DeviceReset")?;
            print!("  resetting...");
            node.execute(true)?;
            println!("OK");
        }
    }
    Ok(())
}
