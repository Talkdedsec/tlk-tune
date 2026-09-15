fn main() {
    println!("cargo:rerun-if-changed=assets/tlk-tune.ico");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/tlk-tune.ico");
        resource.set("ProductName", "tlk-tune");
        resource.set("FileDescription", "Terminal music player");
        resource.set("LegalCopyright", "Talkdedsec - MIT");
        // Needs rc.exe from the Windows SDK. Without it the binary simply
        // ships without an icon, which is not worth failing a build over.
        if let Err(e) = resource.compile() {
            println!("cargo:warning=no version resource: {e}");
        }
    }
}
