use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn get_all_css_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(get_all_css_files(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("css") {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn test_no_unbounded_minmax_2xx() {
    let css_files = get_all_css_files(Path::new("frontend/styles"));
    assert!(!css_files.is_empty(), "CSS files must exist");

    for file in css_files {
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|_| panic!("Failed to read {}", file.display()));
        for (line_idx, line) in content.lines().enumerate() {
            if let Some(pos) = line.find("minmax(2") {
                let slice = &line[pos..];
                // Check if min(100% accompanies minmax(2xxpx
                assert!(
                    slice.contains("min(100%"),
                    "Found unbounded minmax(2xxpx on line {} in {}: '{}'. Must use minmax(min(100%, ...), ...)",
                    line_idx + 1,
                    file.display(),
                    line.trim()
                );
            }
        }
    }
}

#[test]
fn test_100vh_has_100dvh_companion_in_touched_files() {
    let touched_files = [
        "frontend/styles/base/reset.css",
        "frontend/styles/layouts/shell-layout.css",
        "frontend/styles/organisms/stage-mode-view.css",
        "frontend/styles/organisms/agent-chat-panel.css",
        "frontend/styles/events/index.css",
    ];

    for file_path in touched_files {
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", file_path));

        // If file uses 100vh, it must also include 100dvh
        if content.contains("100vh") {
            assert!(
                content.contains("100dvh"),
                "File {} uses 100vh without 100dvh companion",
                file_path
            );
        }

        // If file uses 90vh (modals), it must also use 90dvh
        if content.contains("90vh") {
            assert!(
                content.contains("90dvh"),
                "File {} uses 90vh without 90dvh companion (expected min(90vh, 90dvh))",
                file_path
            );
        }
    }
}

#[test]
fn test_index_html_viewport_and_theme_color() {
    let content = fs::read_to_string("frontend/index.html").expect("read frontend/index.html");
    assert!(
        content.contains("viewport-fit=cover"),
        "frontend/index.html must contain viewport-fit=cover"
    );
    assert!(
        content.contains("name=\"theme-color\""),
        "frontend/index.html must define theme-color meta tag"
    );
}

#[test]
fn test_no_dead_classes_in_events_and_dialog_js() {
    let css_files = get_all_css_files(Path::new("frontend/styles"));
    let mut all_css = String::new();
    for f in css_files {
        all_css.push_str(&fs::read_to_string(f).unwrap());
        all_css.push('\n');
    }

    let js_files = [
        "frontend/js/modules/events/forms.js",
        "frontend/js/modules/events/public-ticket-controller.js",
        "frontend/js/components/organisms/add-item-dialog.js",
    ];

    // Explicit check for known dead classes from audit
    let forbidden_dead_classes_in_js = ["text-input-field", "events-spinner"];

    for js_path in js_files {
        let js_content =
            fs::read_to_string(js_path).unwrap_or_else(|_| panic!("Failed to read {}", js_path));

        for dead_class in forbidden_dead_classes_in_js {
            assert!(
                !js_content.contains(&format!("'{}'", dead_class))
                    && !js_content.contains(&format!("\"{}\"", dead_class)),
                "Dead class '{}' must not be used in {}",
                dead_class,
                js_path
            );
        }

        // Extract class: '...' or class: "..."
        for line in js_content.lines() {
            if let Some(idx) = line.find("class:") {
                let rest = &line[idx + 6..].trim_start();
                if let Some(quote_char) = rest.chars().next() {
                    if quote_char == '\'' || quote_char == '"' {
                        let after_quote = &rest[1..];
                        if let Some(end_idx) = after_quote.find(quote_char) {
                            let class_str = &after_quote[..end_idx];
                            for cls in class_str.split_whitespace() {
                                if cls.is_empty() || cls.contains("${") {
                                    continue;
                                }
                                let selector = format!(".{}", cls);
                                assert!(
                                    all_css.contains(&selector),
                                    "Class '{}' used in {} not found in any CSS file (expected selector '{}')",
                                    cls,
                                    js_path,
                                    selector
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Required classes that must be defined in CSS
    let required_classes = [
        ".text-center",
        ".public-ticket-info-label",
        ".public-ticket-info-value",
        ".add-item-fields",
    ];
    for req in required_classes {
        assert!(all_css.contains(req), "CSS must define selector '{}'", req);
    }
}

#[test]
fn test_touch_targets_minimum_44px() {
    let topbar_css = fs::read_to_string("frontend/styles/organisms/app-topbar.css")
        .expect("read app-topbar.css");

    // .app-topbar-nav-btn must not have min-height 32px, must be >= 44px
    assert!(
        !topbar_css.contains("min-height: 32px;"),
        ".app-topbar-nav-btn must not have min-height: 32px"
    );
    assert!(
        topbar_css.contains(".app-topbar-nav-btn")
            && (topbar_css.contains("min-height: 44px")
                || topbar_css.contains("min-height: var(--touch-target-min")
                || topbar_css.contains("min-height: var(--touch-target-action")),
        ".app-topbar-nav-btn must define touch target min-height >= 44px"
    );
}

#[test]
fn test_media_queries_use_unified_breakpoints() {
    let touched_css = [
        "frontend/styles/views/presentation-detail-view.css",
        "frontend/styles/organisms/stage-mode-view.css",
        "frontend/styles/molecules/quick-add-bar.css",
        "frontend/styles/molecules/playback-controls.css",
        "frontend/styles/organisms/track-item.css",
        "frontend/styles/organisms/app-topbar.css",
        "frontend/styles/organisms/agent-chat-panel.css",
        "frontend/styles/organisms/welcome-header.css",
        "frontend/styles/events/index.css",
        "frontend/styles/layouts/shell-layout.css",
    ];

    let allowed_widths = [
        "480px", "479px", "640px", "639px", "768px", "767px", "769px", "1024px", "1023px",
    ];

    for file_path in touched_css {
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", file_path));

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@media") && !trimmed.contains("prefers-reduced-motion") {
                let mut found_valid = false;
                for allowed in allowed_widths {
                    if trimmed.contains(allowed) {
                        found_valid = true;
                        break;
                    }
                }
                assert!(
                    found_valid,
                    "Disallowed breakpoint in {} on line '{}'. Only 480px, 640px, 768px, 1024px are permitted.",
                    file_path, trimmed
                );
            }
        }
    }
}

#[test]
fn test_frontend_node_unit_tests() {
    let test_files = [
        "frontend/test/door-controller.test.js",
        "frontend/test/events-ui.test.js",
        "frontend/test/public-ticket.test.js",
        "frontend/test/router.test.js",
        "frontend/test/whatsapp.test.js",
    ];

    for test_file in test_files {
        let output = Command::new("node")
            .arg("--test")
            .arg(test_file)
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute node --test {}", test_file));

        assert!(
            output.status.success(),
            "Node test failed for {}:\nSTDOUT:\n{}\nSTDERR:\n{}",
            test_file,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_frontend_root_mjs_integration_tests() {
    let mjs_files = [
        "tests/agent-chat-integration.test.mjs",
        "tests/agent-chat-state.test.mjs",
        "tests/agent-chat-ui-components.test.mjs",
        "tests/wake-lock-race.test.mjs",
    ];

    for mjs in mjs_files {
        let output = Command::new("node")
            .arg(mjs)
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute node {}", mjs));

        assert!(
            output.status.success(),
            "Node mjs test failed for {}:\nSTDOUT:\n{}\nSTDERR:\n{}",
            mjs,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_double_padding_removed_in_events_views() {
    let content =
        fs::read_to_string("frontend/styles/events/index.css").expect("read events/index.css");

    assert!(
        content.contains(".events-catalog-view,\n.events-detail-view {\n  display: flex;\n  flex-direction: column;\n  gap: var(--space-4);\n  max-width: var(--max-content-width);\n  margin: 0 auto;\n  padding: var(--space-3) 0 var(--space-8);"),
        ".events-catalog-view and .events-detail-view must have zero horizontal padding (padding: var(--space-3) 0 var(--space-8))"
    );
    assert!(
        content.contains(".public-ticket-container {\n  max-width: 480px;\n  margin: 0 auto;\n  padding: var(--space-4) 0;"),
        ".public-ticket-container must have zero horizontal padding (padding: var(--space-4) 0)"
    );
    assert!(
        content.contains(".door-container {\n  max-width: 600px;\n  margin: 0 auto;\n  padding: var(--space-3) 0;"),
        ".door-container must have zero horizontal padding (padding: var(--space-3) 0)"
    );
}

#[test]
fn test_door_topbar_flex_wrap_and_event_ellipsis() {
    let content =
        fs::read_to_string("frontend/styles/events/index.css").expect("read events/index.css");

    assert!(
        content.contains(".door-topbar {") && content.contains("flex-wrap: wrap;"),
        ".door-topbar must have flex-wrap: wrap"
    );
    assert!(
        content.contains(".door-header-event {") && content.contains("text-overflow: ellipsis;"),
        ".door-header-event must truncate with text-overflow: ellipsis"
    );
}

#[test]
fn test_welcome_header_send_button_mobile_100_percent() {
    let content = fs::read_to_string("frontend/styles/organisms/welcome-header.css")
        .expect("read welcome-header.css");

    assert!(
        content.contains(".welcome-header-chat-send {") && content.contains("width: 100%;"),
        ".welcome-header-chat-send must have width: 100% on mobile base"
    );
    assert!(
        content.contains("@media (min-width: 640px) {") && content.contains("width: auto;"),
        ".welcome-header-chat-send must revert to width: auto at min-width: 640px"
    );
}

#[test]
fn test_quick_add_bar_mobile_stacking() {
    let content = fs::read_to_string("frontend/styles/molecules/quick-add-bar.css")
        .expect("read quick-add-bar.css");

    assert!(
        content.contains(".quick-add-btn {") && content.contains("flex-direction: column;"),
        ".quick-add-btn must stack icon/label vertically on mobile base"
    );
    assert!(
        content.contains("@media (min-width: 640px) {") && content.contains("flex-direction: row;"),
        ".quick-add-btn must switch to flex-direction: row at min-width: 640px"
    );
}

#[test]
fn test_presentation_detail_title_row_mobile_column() {
    let content = fs::read_to_string("frontend/styles/views/presentation-detail-view.css")
        .expect("read presentation-detail-view.css");

    assert!(
        content.contains(".presentation-title-row {")
            && content.contains("flex-direction: column;"),
        ".presentation-title-row must be column on mobile base"
    );
    assert!(
        content.contains("@media (min-width: 640px) {") && content.contains("flex-direction: row;"),
        ".presentation-title-row must be row at min-width: 640px"
    );
}

#[test]
fn test_stage_mode_exit_button_short_text() {
    let js_content = fs::read_to_string("frontend/js/components/organisms/stage-mode-view.js")
        .expect("read stage-mode-view.js");
    let css_content = fs::read_to_string("frontend/styles/organisms/stage-mode-view.css")
        .expect("read stage-mode-view.css");

    assert!(
        js_content.contains("stage-mode-exit-text-short")
            && js_content.contains("stage-mode-exit-text-full"),
        "stage-mode-view.js must have both short and full exit text elements"
    );
    assert!(
        css_content.contains(".stage-mode-exit-text-short {\n  display: inline;\n}")
            && css_content.contains(".stage-mode-exit-text-full {\n  display: none;\n}"),
        "stage-mode-view.css must show short text and hide full text on mobile base"
    );
}

#[test]
fn test_events_table_cards_mobile() {
    let content =
        fs::read_to_string("frontend/styles/events/index.css").expect("read events/index.css");

    assert!(
        content.contains("@media (max-width: 767px) {")
            && content.contains(".events-table,")
            && content.contains("display: block;"),
        "events-table must transform into cards in @media (max-width: 767px)"
    );
    assert!(
        content.contains("content: attr(data-label);"),
        "events-table must render data-label via ::before"
    );
}
