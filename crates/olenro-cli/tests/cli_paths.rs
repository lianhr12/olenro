use assert_cmd::Command;

#[test]
fn config_dir_points_to_cli_private_directory() {
    let home = tempfile::tempdir().expect("create temp home");

    let mut cmd = Command::cargo_bin("olenro").expect("olenro binary");
    cmd.env("OLENRO_TEST_HOME", home.path())
        .args(["config", "dir"])
        .assert()
        .success()
        .stdout(format!("{}\n", home.path().join(".olenro-cli").display()));
}
