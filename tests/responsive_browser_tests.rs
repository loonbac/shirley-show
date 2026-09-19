use std::process::Command;

#[test]
#[ignore = "Real browser verification with headless Chromium; run on-demand with: cargo test --test responsive_browser_tests -- --ignored"]
fn test_responsive_mobile_real_browser() {
    let status = Command::new("bash")
        .arg("scripts/verify-responsive.sh")
        .status()
        .expect("Failed to execute scripts/verify-responsive.sh");

    assert!(
        status.success(),
        "scripts/verify-responsive.sh failed with status: {:?}",
        status
    );
}
