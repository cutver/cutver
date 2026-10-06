mod common;

use common::Fixture;
use std::process::Command;

#[test]
fn test_e2e_external_subcommand_dispatch_and_exit_code() {
    let fixture = Fixture::new("external-ok");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script_path = fixture.dir.join("cutver-custom");
        std::fs::write(&script_path, "#!/bin/sh\necho \"args: $@\"\nexit 7\n").unwrap();
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();
    }
    #[cfg(windows)]
    {
        std::fs::write(
            fixture.dir.join("cutver-custom.bat"),
            "@echo off\r\necho args: %*\r\nexit /b 7\r\n",
        )
        .unwrap();
    }

    let orig_path = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = fixture.dir.as_os_str().to_os_string();
    let sep = if cfg!(windows) { ";" } else { ":" };
    new_path.push(sep);
    new_path.push(&orig_path);

    let cutver_bin = env!("CARGO_BIN_EXE_cutver");
    let output = Command::new(cutver_bin)
        .args(["custom", "hello", "world"])
        .env("PATH", &new_path)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("args: hello world"), "stdout was: {stdout}");
}

#[test]
fn test_e2e_external_subcommand_not_found() {
    let cutver_bin = env!("CARGO_BIN_EXE_cutver");
    let output = Command::new(cutver_bin)
        .args(["unknownsubcommand12345"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error: unknown subcommand 'unknownsubcommand12345'."),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Where: searching for 'cutver-unknownsubcommand12345' in system PATH"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Fix: ensure 'cutver-unknownsubcommand12345' is installed and present in your system PATH, or run 'cutver --help'."),
        "stderr was: {stderr}"
    );
}
