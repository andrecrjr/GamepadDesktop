use std::{path::PathBuf, process::Command};

fn git_output(manifest: &std::ffi::OsStr, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(manifest)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_path(manifest: &std::ffi::OsStr, path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(git_output(manifest, &["rev-parse", "--git-path", path])?);
    Some(if path.is_absolute() {
        path
    } else {
        PathBuf::from(manifest).join(path)
    })
}

fn main() {
    println!("cargo:rerun-if-changed=assets/gamepad-desktop.ico");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    let manifest = std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets the manifest dir");
    if let Some(head) = git_path(&manifest, "HEAD") {
        println!("cargo:rerun-if-changed={}", head.display());
    }
    if let Some(packed_refs) = git_path(&manifest, "packed-refs") {
        println!("cargo:rerun-if-changed={}", packed_refs.display());
    }
    if let Some(reference) = git_output(&manifest, &["symbolic-ref", "--quiet", "HEAD"])
        && let Some(reference) = git_path(&manifest, &reference)
    {
        println!("cargo:rerun-if-changed={}", reference.display());
    }

    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .or_else(|| git_output(&manifest, &["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=GAMEPAD_DESKTOP_GIT_SHA={commit}");

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
