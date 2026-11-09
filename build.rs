// Embed the application icon in the Windows executable when the Windows SDK
// resource compiler is on PATH. Runtime window icons are set separately.
fn main() {
    println!("cargo:rerun-if-changed=assets/braveamp.rc");
    println!("cargo:rerun-if-changed=assets/braveamp.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let output =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("braveamp.res");
    let compiled = std::process::Command::new("rc.exe")
        .arg("/nologo")
        .arg(format!("/fo{}", output.display()))
        .arg("assets/braveamp.rc")
        .status();
    match compiled {
        Ok(status) if status.success() => println!(
            "cargo:rustc-link-arg-bin=musiccraft={}",
            output.display()
        ),
        _ => println!(
            "cargo:warning=Windows resource compiler not found; runtime icon remains available"
        ),
    }
}
