fn main() {
    println!("cargo:rerun-if-changed=assets/gamepad-desktop.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let target = std::env::var("TARGET").expect("Cargo sets the target triple");
        let mut resource = winres::WindowsResource::new();

        if cfg!(unix) && target.ends_with("windows-gnu") {
            let architecture = target
                .split('-')
                .next()
                .expect("target has an architecture");
            let prefix = format!("{architecture}-w64-mingw32");
            resource
                .set_windres_path(&format!("{prefix}-windres"))
                .set_ar_path(&format!("{prefix}-ar"));
        }

        resource
            .set_icon("assets/gamepad-desktop.ico")
            .compile()
            .expect("failed to compile the Windows app icon");

        // GNU drops the unreferenced resource archive when this package also
        // builds a library. Link the resource object directly into the app.
        if target.ends_with("windows-gnu") {
            let output = std::env::var("OUT_DIR").expect("Cargo sets the build output directory");
            println!("cargo:rustc-link-arg-bin=gamepad-desktop={output}/resource.o");
        }
    }
}
