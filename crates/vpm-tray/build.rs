fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/app.ico");
        res.set("ProductName", "VolumeProfileManager");
        res.set("FileDescription", "VolumeProfileManager TrayApp");
        res.set(
            "LegalCopyright",
            "Copyright (c) VolumeProfileManager Project",
        );
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resource: {e}");
        }
    }
}
