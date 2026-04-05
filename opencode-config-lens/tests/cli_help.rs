use std::process::Command;

#[test]
fn help_should_not_expose_no_color_flag() {
    let exe = env!("CARGO_BIN_EXE_ocl");
    let output = Command::new(exe).arg("--help").output().expect("run help");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(!help.contains("--no-color"));
    assert!(help.contains("--home-dir <HOME_DIR>"));
}
