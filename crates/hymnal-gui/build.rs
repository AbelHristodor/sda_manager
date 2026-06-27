fn main() {
    slint_build::compile("ui/app.slint").unwrap();

    // On Windows, embed the .ico as the executable's icon (shown in Explorer,
    // the taskbar, and Alt-Tab). Absent on other platforms. Paths are relative
    // to this package dir (crates/hymnal-gui), same as the .slint path above.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=../../assets/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");
        res.compile().unwrap();
    }
}
