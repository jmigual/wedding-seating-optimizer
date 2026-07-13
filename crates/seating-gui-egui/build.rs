fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winresource::WindowsResource::new()
            .set_icon("../seating-gui/assets/app-icon.ico")
            .compile()
            .expect("embed Windows icon resource");
    }
}
