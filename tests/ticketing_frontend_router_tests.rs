use std::process::Command;

#[test]
fn test_router_unit_tests() {
    let output = Command::new("node")
        .arg("frontend/test/router.test.js")
        .output()
        .expect("Failed to execute node process for router tests");

    assert!(
        output.status.success(),
        "Router test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_events_ui_integration_tests() {
    let output = Command::new("node")
        .arg("frontend/test/events-ui.test.js")
        .output()
        .expect("Failed to execute node process for events UI tests");

    assert!(
        output.status.success(),
        "Events UI test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_whatsapp_unit_tests() {
    let output = Command::new("node")
        .arg("frontend/test/whatsapp.test.js")
        .output()
        .expect("Failed to execute node process for whatsapp tests");

    assert!(
        output.status.success(),
        "WhatsApp test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_public_ticket_tests() {
    let output = Command::new("node")
        .arg("frontend/test/public-ticket.test.js")
        .output()
        .expect("Failed to execute node process for public ticket tests");

    assert!(
        output.status.success(),
        "Public ticket test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_door_controller_tests() {
    let output = Command::new("node")
        .arg("frontend/test/door-controller.test.js")
        .output()
        .expect("Failed to execute node process for door controller tests");

    assert!(
        output.status.success(),
        "Door controller test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
