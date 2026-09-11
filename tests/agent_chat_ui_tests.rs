use std::process::Command;

#[test]
fn test_agent_chat_state_unit_tests() {
    let output = Command::new("node")
        .arg("tests/agent-chat-state.test.mjs")
        .output()
        .expect("Failed to execute node process for agent chat state tests");

    assert!(
        output.status.success(),
        "Agent chat state test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_agent_chat_ui_components_tests() {
    let output = Command::new("node")
        .arg("tests/agent-chat-ui-components.test.mjs")
        .output()
        .expect("Failed to execute node process for agent chat UI components tests");

    assert!(
        output.status.success(),
        "Agent chat UI components test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_agent_chat_integration_tests() {
    let output = Command::new("node")
        .arg("tests/agent-chat-integration.test.mjs")
        .output()
        .expect("Failed to execute node process for agent chat integration tests");

    assert!(
        output.status.success(),
        "Agent chat integration test failed (exit code {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
