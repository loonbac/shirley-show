use std::process::Command;

#[test]
fn test_wake_lock_race_regression_script() {
    let output = Command::new("node")
        .arg("tests/wake-lock-race.test.mjs")
        .output()
        .expect("Failed to execute node process for wake-lock regression test");

    assert!(
        output.status.success(),
        "Wake-lock regression test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
