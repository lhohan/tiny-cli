fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_test_agent)");

    let has_pi = std::process::Command::new("pi")
        .arg("--version")
        .output()
        .is_ok();

    if has_pi {
        println!("cargo:rustc-cfg=has_test_agent");
    }
}
