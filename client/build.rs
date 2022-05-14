fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use winres::WindowsResource;
        let target_os = std::env::var("CARGO_CFG_TARGET_OS")
            .expect("CARGO_CFG_TARGET_OS should be set by Cargo");
        if target_os == "windows" {
            WindowsResource::new().set_icon("res/icon.ico").compile()?;
        }
    }
    Ok(())
}
