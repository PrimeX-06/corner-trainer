fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico")
            .set("ProductName", "三色定则 · Corner Trainer")
            .set("FileDescription", "魔方角块三色定则训练器")
            .set("LegalCopyright", "MIT License");
        res.compile()
            .expect("Could not compile Windows icon resource");
    }
}
