fn main() -> anyhow::Result<()> {
    // Create an instant camera object with the camera device found first.
    let camera = pylon_shimload::create_first_device()?;

    camera.open()?;

    let pixel_format_node = camera.node_map()?.enum_node("PixelFormat")?;
    for v in pixel_format_node.settable_values()? {
        println!("{}", v);
    }

    Ok(())
}
