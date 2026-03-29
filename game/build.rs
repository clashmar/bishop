fn main() -> std::io::Result<()> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set("FileVersion", "1.0.0.0")
            .set_icon("../editor/windows/Icon.ico")
            .set("FileDescription", "FileDescription")
            .set("ProductVersion", "ProductVersion")
            .set("ProductName", "ProductName")
            .set("OriginalFilename", "game.exe")
            .set("LegalCopyright", "LegalCopyright")
            .set("LegalTrademark", "LegalTrademark")
            .set("CompanyName", "CompanyName")
            .set("Comments", "Comments")
            .set("InternalName", "InternalName")
            .set_version_info(winres::VersionInfo::FILEVERSION, 0x0001000000000000)
            .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);

        res.compile()?;
    }
    Ok(())
}
