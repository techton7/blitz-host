use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use blitz_host_protocol::InspectRequest;
use blitz_host_transport::DebugClient;

fn build_blitz_host_cli() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_manifest = manifest_dir.join("../../Cargo.toml");

    let status = Command::new("cargo")
        .args(["build", "-p", "blitz-host", "--manifest-path"])
        .arg(&workspace_manifest)
        .status()
        .expect("cargo build -p blitz-host must execute");
    assert!(status.success(), "cargo build -p blitz-host must succeed");

    #[cfg(windows)]
    let cli_path = {
        let mut path = manifest_dir.join("../../target/debug/blitz-host");
        path.set_extension("exe");
        path
    };

    #[cfg(not(windows))]
    let cli_path = manifest_dir.join("../../target/debug/blitz-host");

    assert!(
        cli_path.exists(),
        "blitz-host CLI binary must exist at {:?}",
        cli_path
    );
    cli_path
}

fn resolve_native_runner_bin() -> Option<std::path::PathBuf> {
    // 1. Environment variable override
    if let Ok(env_path) = std::env::var("OXIDASE_RUNNER_BIN") {
        let p = std::path::PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Manifest-relative workspace candidates
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join(
            "../../../oxidase/crates/oxidase-native-runner/target/debug/oxidase-native-runner",
        ),
        manifest_dir.join("../../../oxidase/target/debug/oxidase-native-runner"),
        manifest_dir.join("../../target/debug/oxidase-native-runner"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let mut candidate_exe = candidate.clone();
            candidate_exe.set_extension("exe");
            if candidate_exe.exists() {
                return Some(candidate_exe);
            }
        }
    }

    None
}

#[test]
fn test_live_native_runner_attach_and_inspect() {
    let Some(runner_path) = resolve_native_runner_bin() else {
        eprintln!(
            "oxidase-native-runner binary not found via OXIDASE_RUNNER_BIN or relative workspace paths"
        );
        return;
    };

    println!(
        "Starting oxidase-native-runner at {:?} in feature-enabled dev mode (no --debug-control flag)...",
        runner_path
    );
    let mut child = Command::new(&runner_path)
        .arg("--interactive")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn oxidase-native-runner");

    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            for line in BufReader::new(stdout).lines() {
                if let Ok(l) = line {
                    println!("[RUNNER stdout] {l}");
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            for line in BufReader::new(stderr).lines() {
                if let Ok(l) = line {
                    eprintln!("[RUNNER stderr] {l}");
                }
            }
        });
    }

    let pid = child.id();
    println!("Spawned child process with PID: {}", pid);

    // Poll for discovery and attach via PID targeting (wait up to 15 seconds for window mount & document binding)
    let mut client = None;
    for attempt in 1..=30 {
        thread::sleep(Duration::from_millis(500));
        if let Ok(c) = DebugClient::connect_pid(pid) {
            if c.descriptor().primary_document_id.is_some() {
                println!(
                    "Successfully connected to live host with active document via PID {} on attempt #{}",
                    pid, attempt
                );
                client = Some(c);
                break;
            }
            println!(
                "Connected to host PID {} on attempt #{}, waiting for document mount...",
                pid, attempt
            );
        }
    }

    let mut client = match client {
        Some(c) => c,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "Failed to discover and connect to running oxidase-native-runner host within 10s"
            );
        }
    };

    // Verify list_hosts discovers this live PID
    let hosts = blitz_host_transport::list_hosts().expect("list_hosts must succeed");
    assert!(
        hosts.iter().any(|h| h.pid == pid),
        "list_hosts must contain child process PID {pid}"
    );

    let desc = client.descriptor().clone();
    println!("Host Descriptor:");
    println!("  • PID       : {}", desc.pid);
    println!("  • Socket    : {}", desc.socket_path);
    println!("  • Instance  : {}", desc.instance_id);
    println!("  • Renderer  : {}", desc.renderer);

    // Give the UI thread a few frames to finish initial Stylo layout pass
    thread::sleep(Duration::from_millis(500));

    // Execute typed inspect request
    let inspect_result = client
        .inspect(InspectRequest::default())
        .expect("inspect request failed");

    println!("Inspect Result:");
    println!("  • Document ID : {}", inspect_result.document_id);
    println!("  • Root Node ID: {}", inspect_result.root_id);
    println!("  • Total Nodes : {}", inspect_result.node_count);

    assert!(
        inspect_result.node_count > 0,
        "node count must be greater than zero"
    );
    assert_eq!(inspect_result.nodes[0].tag, "#document");

    // Verify presence of real component tags
    let tags: Vec<&str> = inspect_result
        .nodes
        .iter()
        .map(|n| n.tag.as_str())
        .collect();
    println!("  • Observed tags: {:?}", tags);

    let has_button = inspect_result
        .nodes
        .iter()
        .any(|n| n.tag == "button" || n.dom_id.as_deref() == Some("test-interaction-button"));
    assert!(
        has_button,
        "must have found button node in live component tree"
    );

    let nodes_with_bounds = inspect_result
        .nodes
        .iter()
        .filter(|n| n.bounds.is_some())
        .count();
    println!("  • Nodes with layout bounds: {}", nodes_with_bounds);
    assert!(
        nodes_with_bounds > 0,
        "at least some nodes must have layout bounds"
    );

    // Helper to extract text under a node hierarchy (handles direct #text or nested anonymous blocks)
    fn extract_text_under(
        nodes: &[blitz_host_protocol::SemanticNode],
        root_id: u64,
    ) -> Option<String> {
        for node in nodes {
            if node.parent_id == Some(root_id) {
                if let Some(ref txt) = node.text {
                    return Some(txt.clone());
                }
                if let Some(txt) = extract_text_under(nodes, node.id) {
                    return Some(txt);
                }
            }
        }
        None
    }

    let button_node = inspect_result
        .nodes
        .iter()
        .find(|n| n.tag == "button" || n.dom_id.as_deref() == Some("test-interaction-button"))
        .expect("must find button node");
    let button_id = button_node.id;

    let initial_text = extract_text_under(&inspect_result.nodes, button_id);
    println!("  • Initial button text: {:?}", initial_text);
    assert_eq!(
        initial_text.as_deref(),
        Some("Click to Test Event"),
        "initial button text must match expected label"
    );

    // =========================================================================
    // STEP 1: Act (Dispatch Click Action)
    // =========================================================================
    println!("Dispatching click action to button node #{}...", button_id);
    let act_resp = client.click(button_id).expect("click action failed");
    println!(
        "Act Response: success={}, message={:?}",
        act_resp.success, act_resp.message
    );
    assert!(act_resp.success, "act response must indicate success");

    // =========================================================================
    // STEP 2: Settle (Synchronize 2 VSync frames)
    // =========================================================================
    println!("Sending settle request for 2 frames...");
    let settle_resp = client.settle(2).expect("settle request failed");
    println!(
        "Settle Response: settled={}, frames_waited={}, current_frame={}",
        settle_resp.settled, settle_resp.frames_waited, settle_resp.current_frame
    );
    assert!(settle_resp.settled, "settle response must be true");
    assert_eq!(settle_resp.frames_waited, 2);

    // =========================================================================
    // STEP 3: Definitive Proof: State Mutation in Live Semantic Tree
    // (Proof is observed state change, NOT just frame count)
    // =========================================================================
    let post_click_inspect = client
        .inspect(InspectRequest::default())
        .expect("post-click inspect failed");
    let updated_text = extract_text_under(&post_click_inspect.nodes, button_id);
    println!(
        "  • Observed button text after click + settle(2): {:?}",
        updated_text
    );
    assert_eq!(
        updated_text.as_deref(),
        Some("Clicked 1 times"),
        "CRITICAL PROOF: Button label must transition from 'Click to Test Event' to 'Clicked 1 times'"
    );

    // =========================================================================
    // STEP 4: Second Click & settle_until Verification
    // =========================================================================
    println!("Dispatching second click action to verify continuous reactivity...");
    let act2_resp = client.click(button_id).expect("second click action failed");
    assert!(act2_resp.success);

    println!("Testing settle_until helper waiting for 'Clicked 2 times'...");
    let settled_snapshot = client
        .settle_until(Duration::from_secs(5), |snap| {
            extract_text_under(&snap.nodes, button_id).as_deref() == Some("Clicked 2 times")
        })
        .expect("settle_until timed out waiting for 'Clicked 2 times'");

    let final_text = extract_text_under(&settled_snapshot.nodes, button_id);
    println!(
        "  • Final button text after second click + settle_until: {:?}",
        final_text
    );
    assert_eq!(
        final_text.as_deref(),
        Some("Clicked 2 times"),
        "Second click must update state to 'Clicked 2 times'"
    );

    // =========================================================================
    // STEP 5: Locate Input Node & Verify Initial Focus State
    // =========================================================================
    let input_node = settled_snapshot
        .nodes
        .iter()
        .find(|n| n.tag == "input" || n.dom_id.as_deref() == Some("test-input"))
        .expect("must find input node in live component tree");
    let input_id = input_node.id;
    println!(
        "  • Found input node #{} (dom_id: {:?})",
        input_id, input_node.dom_id
    );
    assert_ne!(
        input_node.focused,
        Some(true),
        "input node must not be focused initially"
    );

    // =========================================================================
    // STEP 6: Act (Dispatch Focus Action) & Settle
    // =========================================================================
    println!("Dispatching focus action to input node #{}...", input_id);
    let focus_resp = client.focus(input_id).expect("focus action failed");
    println!(
        "Focus Response: success={}, message={:?}",
        focus_resp.success, focus_resp.message
    );
    assert!(focus_resp.success, "focus response must indicate success");

    println!("Sending settle request for 2 frames after focus...");
    let settle_focus = client.settle(2).expect("settle request after focus failed");
    assert!(settle_focus.settled);

    let post_focus_inspect = client
        .inspect(InspectRequest::default())
        .expect("post-focus inspect failed");

    let post_focus_input = post_focus_inspect
        .nodes
        .iter()
        .find(|n| n.id == input_id)
        .expect("input node must still exist");

    println!(
        "  • Observed focused_node_id: {:?}",
        post_focus_inspect.focused_node_id
    );
    println!("  • Observed input.focused: {:?}", post_focus_input.focused);
    assert_eq!(
        post_focus_inspect.focused_node_id,
        Some(input_id),
        "CRITICAL PROOF: focused_node_id in InspectResponse must equal input_id"
    );
    assert_eq!(
        post_focus_input.focused,
        Some(true),
        "CRITICAL PROOF: input node SemanticNode must have focused == Some(true)"
    );

    // Verify Dioxus rendered the conditional span#focus-indicator
    let has_focus_indicator = post_focus_inspect.nodes.iter().any(|n| {
        n.dom_id.as_deref() == Some("focus-indicator")
            || extract_text_under(&post_focus_inspect.nodes, n.id).as_deref() == Some("FOCUSED")
    });
    println!(
        "  • Conditional focus indicator rendered: {}",
        has_focus_indicator
    );
    assert!(
        has_focus_indicator,
        "CRITICAL PROOF: Dioxus must render focus indicator span in response to onfocus event"
    );

    // =========================================================================
    // STEP 7: Act (Dispatch SetValue Action) & Settle
    // =========================================================================
    let test_typed_str = "Hello from blitz-host live test!";
    println!(
        "Dispatching set_value action to input node #{} with value {:?}...",
        input_id, test_typed_str
    );
    let set_val_resp = client
        .set_value(input_id, test_typed_str)
        .expect("set_value action failed");
    println!(
        "SetValue Response: success={}, message={:?}",
        set_val_resp.success, set_val_resp.message
    );
    assert!(
        set_val_resp.success,
        "set_value response must indicate success"
    );

    println!("Sending settle request for 2 frames after set_value...");
    let settle_val = client
        .settle(2)
        .expect("settle request after set_value failed");
    assert!(settle_val.settled);

    let post_val_inspect = client
        .inspect(InspectRequest::default())
        .expect("post-set_value inspect failed");

    let post_val_input = post_val_inspect
        .nodes
        .iter()
        .find(|n| n.id == input_id)
        .expect("input node must exist");
    println!("  • Input node text/value: {:?}", post_val_input.text);
    assert_eq!(
        post_val_input.text.as_deref(),
        Some(test_typed_str),
        "CRITICAL PROOF: Input element text/value must match set value"
    );

    // Verify Dioxus oninput handler updated reactive signal rendering in p#typed-text
    let typed_p_node = post_val_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("typed-text"))
        .expect("must find p#typed-text in semantic tree");
    let typed_p_text = extract_text_under(&post_val_inspect.nodes, typed_p_node.id);
    println!("  • Observed p#typed-text: {:?}", typed_p_text);
    let expected_p_text = format!("Typed: {test_typed_str}");
    assert_eq!(
        typed_p_text.as_deref(),
        Some(expected_p_text.as_str()),
        "CRITICAL PROOF: oninput event must trigger Dioxus reactivity and update p#typed-text"
    );

    // =========================================================================
    // STEP 8: Continuous Reactivity - Second SetValue with settle_until
    // =========================================================================
    let second_test_str = "Continuous reactivity 42";
    println!("Dispatching second set_value: {:?}", second_test_str);
    let set_val2_resp = client
        .set_value(input_id, second_test_str)
        .expect("second set_value failed");
    assert!(set_val2_resp.success);

    let expected_p_text2 = format!("Typed: {second_test_str}");
    let settled_val_snapshot = client
        .settle_until(Duration::from_secs(5), |snap| {
            if let Some(p) = snap
                .nodes
                .iter()
                .find(|n| n.dom_id.as_deref() == Some("typed-text"))
            {
                extract_text_under(&snap.nodes, p.id).as_deref() == Some(expected_p_text2.as_str())
            } else {
                false
            }
        })
        .expect("settle_until timed out waiting for second typed text");

    let final_typed_p = settled_val_snapshot
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("typed-text"))
        .unwrap();
    let final_typed_text = extract_text_under(&settled_val_snapshot.nodes, final_typed_p.id);
    println!(
        "  • Final typed text after settle_until: {:?}",
        final_typed_text
    );
    assert_eq!(final_typed_text.as_deref(), Some(expected_p_text2.as_str()));

    // =========================================================================
    // STEP 9: Live Core Keyboard Lane Verification
    // =========================================================================
    println!("Testing live core keyboard lane against running native host...");

    // 9.1: Button activation via Enter
    let focus_btn_resp = client.focus(button_id).expect("focus button failed");
    assert!(focus_btn_resp.success);
    let _ = client.settle(2).expect("settle failed");

    println!("Dispatching Enter key to focused button...");
    let enter_resp = client.enter().expect("enter key dispatch failed");
    assert!(enter_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_enter_inspect = client.inspect(InspectRequest::default()).unwrap();
    let text_after_enter = extract_text_under(&post_enter_inspect.nodes, button_id);
    println!("  • Button label after Enter: {:?}", text_after_enter);
    assert_eq!(
        text_after_enter.as_deref(),
        Some("Clicked 3 times"),
        "CRITICAL PROOF: Enter key on button must trigger onclick and increment count"
    );

    // 9.2: Button activation via Space
    println!("Dispatching Space key to focused button...");
    let space_resp = client.space().expect("space key dispatch failed");
    assert!(space_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_space_inspect = client.inspect(InspectRequest::default()).unwrap();
    let text_after_space = extract_text_under(&post_space_inspect.nodes, button_id);
    println!("  • Button label after Space: {:?}", text_after_space);
    assert_eq!(
        text_after_space.as_deref(),
        Some("Clicked 4 times"),
        "CRITICAL PROOF: Space key on button must trigger onclick and increment count"
    );

    // 9.3: Focus traversal via Shift+Tab back to input
    println!("Dispatching Shift+Tab to focus input...");
    let stab_resp = client.shift_tab().expect("shift_tab key dispatch failed");
    assert!(stab_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_stab_inspect = client.inspect(InspectRequest::default()).unwrap();
    println!(
        "  • Focused node after Shift+Tab: {:?}",
        post_stab_inspect.focused_node_id
    );
    assert_eq!(
        post_stab_inspect.focused_node_id,
        Some(input_id),
        "CRITICAL PROOF: Shift+Tab must navigate focus to input"
    );

    // 9.4: Text editing - Select All (Ctrl/Cmd + A) and overwrite
    println!("Testing Select All (Cmd/Ctrl + A) and overwrite on input...");
    let sel_all_resp = client
        .select_all(Some(input_id))
        .expect("select_all failed");
    assert!(sel_all_resp.success);

    // Type "K" to overwrite
    let key_k_resp = client
        .key(None, Some(input_id), "K")
        .expect("typing K failed");
    assert!(key_k_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_overwrite_inspect = client.inspect(InspectRequest::default()).unwrap();
    let typed_p_after = post_overwrite_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("typed-text"))
        .expect("must find typed-text");
    let typed_text_after = extract_text_under(&post_overwrite_inspect.nodes, typed_p_after.id);
    println!(
        "  • Typed text after Cmd/Ctrl+A + 'K': {:?}",
        typed_text_after
    );
    assert_eq!(
        typed_text_after.as_deref(),
        Some("Typed: K"),
        "CRITICAL PROOF: Select all and typing 'K' must overwrite previous value"
    );

    // 9.5: Text editing - Type additional character and delete with Backspace
    let key_q_resp = client
        .key(None, Some(input_id), "Q")
        .expect("typing Q failed");
    assert!(key_q_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_q_inspect = client.inspect(InspectRequest::default()).unwrap();
    let typed_p_q = post_q_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("typed-text"))
        .unwrap();
    assert_eq!(
        extract_text_under(&post_q_inspect.nodes, typed_p_q.id).as_deref(),
        Some("Typed: KQ")
    );

    let bs_resp = client.backspace(Some(input_id)).expect("backspace failed");
    assert!(bs_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_bs_inspect = client.inspect(InspectRequest::default()).unwrap();
    let typed_p_bs = post_bs_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("typed-text"))
        .unwrap();
    println!(
        "  • Typed text after Backspace: {:?}",
        extract_text_under(&post_bs_inspect.nodes, typed_p_bs.id)
    );
    assert_eq!(
        extract_text_under(&post_bs_inspect.nodes, typed_p_bs.id).as_deref(),
        Some("Typed: K"),
        "CRITICAL PROOF: Backspace must delete preceding character"
    );

    // 9.6: Escape clears focus
    println!("Dispatching Escape to clear focus...");
    let esc_resp = client.escape().expect("escape failed");
    assert!(esc_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_esc_inspect = client.inspect(InspectRequest::default()).unwrap();
    let post_esc_input = post_esc_inspect
        .nodes
        .iter()
        .find(|n| n.id == input_id)
        .unwrap();
    println!(
        "  • Input focused after Escape: {:?}",
        post_esc_input.focused
    );
    assert_ne!(
        post_esc_input.focused,
        Some(true),
        "CRITICAL PROOF: Escape must clear input focus"
    );

    // =========================================================================
    // STEP 10: Live Core Mouse / Pointer / Wheel Lane Verification
    // =========================================================================
    println!("Testing live core mouse / pointer / wheel lane against running native host...");

    let card_node = post_esc_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("mouse-test-card"))
        .expect("must find #mouse-test-card in semantic tree");
    let card_id = card_node.id;

    let scroll_container_node = post_esc_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("test-scroll-container"))
        .expect("must find #test-scroll-container in semantic tree");
    let scroll_id = scroll_container_node.id;

    // 10.1: Pointer Move / Hover
    println!(
        "Testing live pointer move / hover on #mouse-test-card (node #{})...",
        card_id
    );
    let move_resp = client.hover(card_id).expect("hover action failed");
    assert!(move_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_hover_inspect = client.inspect(InspectRequest::default()).unwrap();
    println!(
        "  • Observed hover_node_id: {:?}",
        post_hover_inspect.hover_node_id
    );
    let post_hover_card = post_hover_inspect
        .nodes
        .iter()
        .find(|n| n.id == card_id)
        .expect("card must exist");

    let is_card_or_child_hovered_target = match post_hover_inspect.hover_node_id {
        Some(hid) => hid == card_id || post_hover_card.children.contains(&hid),
        None => false,
    };
    assert!(
        is_card_or_child_hovered_target,
        "CRITICAL PROOF: hover_node_id in InspectResponse must equal card_id or its child"
    );

    let is_hover_state_active = post_hover_card.hovered == Some(true)
        || post_hover_inspect
            .nodes
            .iter()
            .any(|n| n.parent_id == Some(card_id) && n.hovered == Some(true));
    println!("  • Card hovered state: {:?}", post_hover_card.hovered);
    assert!(
        is_hover_state_active,
        "CRITICAL PROOF: Card or its inner element must report hovered == Some(true)"
    );

    let hover_status_node = post_hover_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("hover-status"))
        .expect("must find hover-status");
    let hover_text = extract_text_under(&post_hover_inspect.nodes, hover_status_node.id);
    println!("  • Observed #hover-status: {:?}", hover_text);
    assert_eq!(
        hover_text.as_deref(),
        Some("HOVERED"),
        "CRITICAL PROOF: onmouseenter event must trigger Dioxus reactivity and render HOVERED"
    );

    // 10.2: Pointer Down
    println!("Testing live pointer down on #mouse-test-card...");
    let down_resp = client
        .mouse_down(None, Some(card_id), None, Some("left"), None)
        .expect("mouse_down failed");
    assert!(down_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_down_inspect = client.inspect(InspectRequest::default()).unwrap();
    let post_down_card = post_down_inspect
        .nodes
        .iter()
        .find(|n| n.id == card_id)
        .expect("card must exist");
    let is_card_or_child_active = post_down_card.active == Some(true)
        || post_down_inspect
            .nodes
            .iter()
            .any(|n| n.parent_id == Some(card_id) && n.active == Some(true));
    println!("  • Card active state: {:?}", post_down_card.active);
    assert!(
        is_card_or_child_active,
        "CRITICAL PROOF: Card or its inner element must report active == Some(true) on pointer down"
    );

    let press_status_node = post_down_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("pressed-status"))
        .expect("must find pressed-status");
    let press_text = extract_text_under(&post_down_inspect.nodes, press_status_node.id);
    println!("  • Observed #pressed-status: {:?}", press_text);
    assert_eq!(
        press_text.as_deref(),
        Some("PRESSED"),
        "CRITICAL PROOF: onpointerdown event must trigger Dioxus reactivity and render PRESSED"
    );

    // 10.3: Pointer Up
    println!("Testing live pointer up on #mouse-test-card...");
    let up_resp = client
        .mouse_up(None, Some(card_id), None, Some("left"), None)
        .expect("mouse_up failed");
    assert!(up_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_up_inspect = client.inspect(InspectRequest::default()).unwrap();
    let post_up_card = post_up_inspect
        .nodes
        .iter()
        .find(|n| n.id == card_id)
        .expect("card must exist");
    assert_ne!(
        post_up_card.active,
        Some(true),
        "CRITICAL PROOF: Card SemanticNode active state must clear on pointer up"
    );

    let press_status_node_up = post_up_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("pressed-status"))
        .expect("must find pressed-status");
    let press_text_up = extract_text_under(&post_up_inspect.nodes, press_status_node_up.id);
    println!("  • Observed #pressed-status after up: {:?}", press_text_up);
    assert_eq!(
        press_text_up.as_deref(),
        Some("RELEASED"),
        "CRITICAL PROOF: onpointerup event must trigger Dioxus reactivity and render RELEASED"
    );

    // 10.4: Composed Drag Sequence
    println!("Testing composed drag sequence from card to input...");
    let drag_resp = client
        .drag(card_id, input_id)
        .expect("drag sequence failed");
    assert!(drag_resp.success);
    let _ = client.settle(2).expect("settle failed");

    // 10.5: Wheel / Scroll
    println!(
        "Testing wheel scroll on #test-scroll-container (node #{})...",
        scroll_id
    );
    let wheel_resp = client
        .scroll(scroll_id, 45.0)
        .expect("scroll action failed");
    assert!(wheel_resp.success);
    let _ = client.settle(2).expect("settle failed");

    let post_wheel_inspect = client.inspect(InspectRequest::default()).unwrap();
    let scroll_status_node = post_wheel_inspect
        .nodes
        .iter()
        .find(|n| n.dom_id.as_deref() == Some("scroll-status"))
        .expect("must find scroll-status");
    let scroll_status_text = extract_text_under(&post_wheel_inspect.nodes, scroll_status_node.id);
    println!("  • Observed #scroll-status: {:?}", scroll_status_text);
    assert_eq!(
        scroll_status_text.as_deref(),
        Some("Scroll Y: 45"),
        "CRITICAL PROOF: onwheel event must update Dioxus reactivity and report new scroll position"
    );

    // =========================================================================
    // STEP 11: Visual Capture Proof (Live Visual Screenshot PNG)
    // =========================================================================
    println!("Capturing visual screenshot from live native host...");
    let proof_artifact_path = std::path::PathBuf::from("target/live_proof_artifact.png");
    let _ = std::fs::remove_file(&proof_artifact_path);
    let cap_resp = client
        .capture(&proof_artifact_path)
        .expect("capture request failed");
    println!(
        "Capture Response: success={}, dimensions={}x{}, format={:?}, bytes={}, file={}",
        cap_resp.success,
        cap_resp.width,
        cap_resp.height,
        cap_resp.format,
        cap_resp.bytes,
        cap_resp.file_path
    );
    assert!(cap_resp.success, "capture response must indicate success");
    assert!(
        cap_resp.width > 0,
        "captured width must be greater than zero"
    );
    assert!(
        cap_resp.height > 0,
        "captured height must be greater than zero"
    );
    assert_eq!(cap_resp.format, "png");
    assert_eq!(cap_resp.node_id, None);
    assert!(
        cap_resp.bytes > 100,
        "captured PNG bytes must be non-trivial"
    );
    assert!(proof_artifact_path.exists());
    let png_bytes = std::fs::read(&proof_artifact_path).expect("file must exist on disk");
    assert_eq!(
        &png_bytes[0..4],
        &[0x89, b'P', b'N', b'G'],
        "CRITICAL PROOF: Captured visual payload must start with valid PNG header magic bytes"
    );

    // =========================================================================
    // STEP 12: Subtree / Node-Level Visual Capture Proof (Cropped PNG)
    // =========================================================================
    println!(
        "Testing subtree / node-level visual capture on #mouse-test-card (node #{})...",
        card_id
    );
    let node_artifact_path = std::path::PathBuf::from("target/live_proof_node_card.png");
    let _ = std::fs::remove_file(&node_artifact_path);
    let node_cap_resp = client
        .capture_node(card_id, &node_artifact_path)
        .expect("node capture request failed");
    println!(
        "Node Capture Response: success={}, dimensions={}x{}, format={:?}, bytes={}, node_id={:?}",
        node_cap_resp.success,
        node_cap_resp.width,
        node_cap_resp.height,
        node_cap_resp.format,
        node_cap_resp.bytes,
        node_cap_resp.node_id
    );
    assert!(
        node_cap_resp.success,
        "node capture response must indicate success"
    );
    assert_eq!(
        node_cap_resp.node_id,
        Some(card_id),
        "response node_id must match requested node"
    );
    assert!(
        node_cap_resp.width > 0,
        "cropped width must be greater than zero"
    );
    assert!(
        node_cap_resp.height > 0,
        "cropped height must be greater than zero"
    );
    assert!(
        node_cap_resp.width < cap_resp.width,
        "CRITICAL PROOF: cropped node width ({}) must be strictly smaller than full-window width ({})",
        node_cap_resp.width,
        cap_resp.width
    );
    assert!(
        node_cap_resp.height < cap_resp.height,
        "CRITICAL PROOF: cropped node height ({}) must be strictly smaller than full-window height ({})",
        node_cap_resp.height,
        cap_resp.height
    );
    assert!(node_artifact_path.exists());
    let node_file_size = std::fs::metadata(&node_artifact_path).unwrap().len();
    assert_eq!(node_file_size as usize, node_cap_resp.bytes);

    // Also test node capture on #test-interaction-button
    println!(
        "Testing node-level visual capture on #test-interaction-button (node #{})...",
        button_id
    );
    let btn_artifact_path = std::path::PathBuf::from("target/live_proof_btn.png");
    let _ = std::fs::remove_file(&btn_artifact_path);
    let btn_cap_resp = client
        .capture_node(button_id, &btn_artifact_path)
        .expect("button capture failed");
    assert!(btn_cap_resp.success);
    assert_eq!(btn_cap_resp.node_id, Some(button_id));
    assert!(btn_cap_resp.width < cap_resp.width);
    assert!(btn_cap_resp.height < cap_resp.height);
    assert!(btn_artifact_path.exists());

    // Test capture on non-existent node reports honest failure
    println!("Testing node-level visual capture on non-existent node #9999999...");
    let invalid_cap_resp = client
        .capture_node(9999999, "target/nonexistent.png")
        .expect("request must succeed over IPC");
    assert!(
        !invalid_cap_resp.success,
        "capture on non-existent node must report success=false"
    );
    assert!(
        invalid_cap_resp
            .message
            .as_deref()
            .unwrap()
            .contains("not found in document")
    );

    // =========================================================================
    // STEP 13: Live blitz-host CLI Verification (Always-on JSON, mandatory -o, compound key, mouse namespace)
    // =========================================================================
    let cli_path = build_blitz_host_cli();
    println!(
        "Testing blitz-host CLI against live native host (PID {})...",
        pid
    );

    // 13.1: list outputs clean JSON
    let list_output = Command::new(&cli_path)
        .arg("list")
        .output()
        .expect("blitz-host list failed");
    assert!(list_output.status.success());
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let list_json: serde_json::Value =
        serde_json::from_str(list_stdout.trim()).expect("blitz-host list must output valid JSON");
    assert!(list_json.is_array());
    let found_pid = list_json
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["pid"] == pid);
    assert!(found_pid, "blitz-host list JSON must contain live host PID");

    // 13.2a: full-window inspect outputs clean JSON
    let inspect_output = Command::new(&cli_path)
        .args(["inspect", "--pid", &pid.to_string()])
        .output()
        .expect("blitz-host inspect failed");
    assert!(inspect_output.status.success());
    let inspect_stdout = String::from_utf8_lossy(&inspect_output.stdout);
    let inspect_json: serde_json::Value = serde_json::from_str(inspect_stdout.trim())
        .expect("blitz-host inspect must output valid JSON");
    assert_eq!(inspect_json["documentId"], 1);

    // 13.2b: subtree inspect with positional [NODE_ID] outputs compact subtree starting at target node
    let subtree_pos_output = Command::new(&cli_path)
        .args(["inspect", &card_id.to_string(), "--pid", &pid.to_string()])
        .output()
        .expect("blitz-host inspect [NODE_ID] failed");
    assert!(subtree_pos_output.status.success());
    let subtree_pos_stdout = String::from_utf8_lossy(&subtree_pos_output.stdout);
    let subtree_pos_json: serde_json::Value = serde_json::from_str(subtree_pos_stdout.trim())
        .expect("blitz-host inspect positional subtree must output valid JSON");
    assert_eq!(subtree_pos_json["rootId"], card_id);
    assert!(
        subtree_pos_json["nodes"].as_array().unwrap().len()
            <= inspect_json["nodes"].as_array().unwrap().len()
    );

    // 13.2c: subtree inspect with explicit --node <NODE_ID>
    let subtree_flag_output = Command::new(&cli_path)
        .args([
            "inspect",
            "--node",
            &card_id.to_string(),
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host inspect --node failed");
    assert!(subtree_flag_output.status.success());
    let subtree_flag_stdout = String::from_utf8_lossy(&subtree_flag_output.stdout);
    let subtree_flag_json: serde_json::Value = serde_json::from_str(subtree_flag_stdout.trim())
        .expect("blitz-host inspect flag subtree must output valid JSON");
    assert_eq!(subtree_flag_json["rootId"], card_id);

    // 13.3: key with compound specifier (cmd+a)
    let key_output = Command::new(&cli_path)
        .args([
            "key",
            "cmd+a",
            "--node",
            &input_id.to_string(),
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host key failed");
    assert!(key_output.status.success());
    let key_stdout = String::from_utf8_lossy(&key_output.stdout);
    let key_json: serde_json::Value =
        serde_json::from_str(key_stdout.trim()).expect("blitz-host key must output valid JSON");
    assert_eq!(key_json["success"], true);

    // 13.4a: mouse namespace action (mouse move)
    let mouse_output = Command::new(&cli_path)
        .args([
            "mouse",
            "move",
            &card_id.to_string(),
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host mouse move failed");
    assert!(mouse_output.status.success());
    let mouse_stdout = String::from_utf8_lossy(&mouse_output.stdout);
    let mouse_json: serde_json::Value = serde_json::from_str(mouse_stdout.trim())
        .expect("blitz-host mouse move must output valid JSON");
    assert_eq!(mouse_json["success"], true);

    // 13.4b: mouse namespace action (mouse click)
    let mouse_click_output = Command::new(&cli_path)
        .args([
            "mouse",
            "click",
            &button_id.to_string(),
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host mouse click failed");
    assert!(mouse_click_output.status.success());
    let mouse_click_stdout = String::from_utf8_lossy(&mouse_click_output.stdout);
    let mouse_click_json: serde_json::Value = serde_json::from_str(mouse_click_stdout.trim())
        .expect("blitz-host mouse click must output valid JSON");
    assert_eq!(mouse_click_json["success"], true);

    // 13.4c: top-level pointer command without mouse namespace MUST fail (zero fallback)
    let fail_click_output = Command::new(&cli_path)
        .args(["click", &button_id.to_string(), "--pid", &pid.to_string()])
        .output()
        .expect("top-level click command failed to execute");
    assert_eq!(
        fail_click_output.status.code(),
        Some(1),
        "top-level click must exit with code 1"
    );
    let fail_click_stderr = String::from_utf8_lossy(&fail_click_output.stderr);
    assert!(
        fail_click_stderr.contains("belongs under the 'mouse' namespace"),
        "stderr must direct user to use 'blitz-host mouse click'"
    );

    let fail_move_output = Command::new(&cli_path)
        .args(["move", &card_id.to_string(), "--pid", &pid.to_string()])
        .output()
        .expect("top-level move command failed to execute");
    assert_eq!(
        fail_move_output.status.code(),
        Some(1),
        "top-level move must exit with code 1"
    );
    let fail_move_stderr = String::from_utf8_lossy(&fail_move_output.stderr);
    assert!(
        fail_move_stderr.contains("belongs under the 'mouse' namespace"),
        "stderr must direct user to use 'blitz-host mouse move'"
    );

    // 13.5: capture without -o MUST fail with exit code 1
    let fail_cap_output = Command::new(&cli_path)
        .args(["capture", &card_id.to_string(), "--pid", &pid.to_string()])
        .output()
        .expect("capture execution failed");
    assert_eq!(
        fail_cap_output.status.code(),
        Some(1),
        "capture without -o must exit with 1"
    );
    let fail_stderr = String::from_utf8_lossy(&fail_cap_output.stderr);
    assert!(
        fail_stderr.contains("Output path is required"),
        "stderr must explain -o requirement"
    );

    // 13.6: full-window capture with -o produces valid metadata JSON (no base64) and writes PNG artifact
    let cli_full_path = "target/cli_proof_full_window.png";
    let _ = std::fs::remove_file(cli_full_path);

    let full_cap_output = Command::new(&cli_path)
        .args(["capture", "-o", cli_full_path, "--pid", &pid.to_string()])
        .output()
        .expect("blitz-host capture full-window with -o failed");
    assert!(
        full_cap_output.status.success(),
        "blitz-host capture full-window must succeed"
    );
    let full_cap_stdout = String::from_utf8_lossy(&full_cap_output.stdout);
    let full_cap_json: serde_json::Value = serde_json::from_str(full_cap_stdout.trim())
        .expect("blitz-host capture full-window must output valid metadata JSON");
    assert_eq!(full_cap_json["success"], true);
    assert!(
        full_cap_json["filePath"]
            .as_str()
            .unwrap()
            .ends_with(cli_full_path)
    );
    assert_eq!(full_cap_json["width"], 800);
    assert_eq!(full_cap_json["height"], 600);
    assert!(
        full_cap_json["nodeId"].is_null(),
        "full-window capture must have null nodeId"
    );
    assert!(
        full_cap_json.get("dataBase64").is_none(),
        "CRITICAL PROOF: metadata JSON must NOT contain dataBase64"
    );
    assert!(
        std::path::Path::new(cli_full_path).exists(),
        "Output full-window PNG file must exist on disk"
    );

    // 13.7: node capture with positional <NODE_ID> and -o
    let cli_artifact_path = "target/cli_proof_node_card.png";
    let _ = std::fs::remove_file(cli_artifact_path);

    let cap_output = Command::new(&cli_path)
        .args([
            "capture",
            &card_id.to_string(),
            "-o",
            cli_artifact_path,
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host capture with -o failed");
    assert!(
        cap_output.status.success(),
        "blitz-host capture with -o must succeed"
    );
    let cap_stdout = String::from_utf8_lossy(&cap_output.stdout);
    let cap_json: serde_json::Value = serde_json::from_str(cap_stdout.trim())
        .expect("blitz-host capture must output valid metadata JSON");
    assert_eq!(cap_json["success"], true);
    assert!(
        cap_json["filePath"]
            .as_str()
            .unwrap()
            .ends_with(cli_artifact_path)
    );
    assert_eq!(cap_json["nodeId"], card_id);
    assert!(
        cap_json.get("dataBase64").is_none(),
        "CRITICAL PROOF: metadata JSON must NOT contain dataBase64"
    );
    assert!(
        std::path::Path::new(cli_artifact_path).exists(),
        "Output node PNG file must exist on disk"
    );

    // 13.8: node capture with explicit --node <NODE_ID> and -o
    let cli_flag_path = "target/cli_proof_node_card_flag.png";
    let _ = std::fs::remove_file(cli_flag_path);

    let cap_flag_output = Command::new(&cli_path)
        .args([
            "capture",
            "--node",
            &card_id.to_string(),
            "-o",
            cli_flag_path,
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host capture --node with -o failed");
    assert!(
        cap_flag_output.status.success(),
        "blitz-host capture --node with -o must succeed"
    );
    let cap_flag_stdout = String::from_utf8_lossy(&cap_flag_output.stdout);
    let cap_flag_json: serde_json::Value = serde_json::from_str(cap_flag_stdout.trim())
        .expect("blitz-host capture --node must output valid metadata JSON");
    assert_eq!(cap_flag_json["success"], true);
    assert!(
        cap_flag_json["filePath"]
            .as_str()
            .unwrap()
            .ends_with(cli_flag_path)
    );
    assert_eq!(cap_flag_json["nodeId"], card_id);
    assert!(
        cap_flag_json.get("dataBase64").is_none(),
        "CRITICAL PROOF: metadata JSON must NOT contain dataBase64"
    );
    assert!(
        std::path::Path::new(cli_flag_path).exists(),
        "Output node flag PNG file must exist on disk"
    );

    println!(
        "  • blitz-host CLI verified: Always-on JSON, compound keys, mouse namespace, full-window & node mandatory -o metadata JSON!"
    );

    // -----------------------------------------------------------------
    // STEP 14: CSS Selector targeting proof across CLI and Client
    // -----------------------------------------------------------------
    println!("\n[STEP 14] Proving querySelector-style CSS selector targeting...");

    // 14.1: Client API inspect_target with CSS selector
    let sel_inspect = client
        .inspect_target("#test-input")
        .expect("inspect_target with selector should succeed");
    assert!(
        sel_inspect.node_count > 0,
        "inspect_target must return matched subtree"
    );
    assert_eq!(sel_inspect.nodes[0].dom_id.as_deref(), Some("test-input"));

    // 14.2: CLI mouse click via CSS selector
    let sel_click_output = Command::new(&cli_path)
        .args([
            "mouse",
            "click",
            "#test-interaction-button",
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host mouse click with selector failed");
    assert!(
        sel_click_output.status.success(),
        "blitz-host mouse click #test-interaction-button must succeed"
    );
    let sel_click_stdout = String::from_utf8_lossy(&sel_click_output.stdout);
    let sel_click_json: serde_json::Value = serde_json::from_str(sel_click_stdout.trim())
        .expect("blitz-host mouse click must output valid JSON");
    assert_eq!(sel_click_json["success"], true);

    // 14.3: CLI focus via CSS selector
    let sel_focus_output = Command::new(&cli_path)
        .args(["focus", "#test-input", "--pid", &pid.to_string()])
        .output()
        .expect("blitz-host focus with selector failed");
    assert!(
        sel_focus_output.status.success(),
        "blitz-host focus #test-input must succeed"
    );
    let sel_focus_stdout = String::from_utf8_lossy(&sel_focus_output.stdout);
    let sel_focus_json: serde_json::Value = serde_json::from_str(sel_focus_stdout.trim())
        .expect("blitz-host focus must output valid JSON");
    assert_eq!(sel_focus_json["success"], true);

    // 14.4: CLI set-value via CSS selector
    let test_text = "Typed via CSS selector proof!";
    let sel_setval_output = Command::new(&cli_path)
        .args([
            "set-value",
            "#test-input",
            test_text,
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host set-value with selector failed");
    assert!(
        sel_setval_output.status.success(),
        "blitz-host set-value #test-input must succeed"
    );
    let sel_setval_stdout = String::from_utf8_lossy(&sel_setval_output.stdout);
    let sel_setval_json: serde_json::Value = serde_json::from_str(sel_setval_stdout.trim())
        .expect("blitz-host set-value must output valid JSON");
    assert_eq!(sel_setval_json["success"], true);

    // 14.5: CLI inspect subtree via CSS selector
    let sel_inspect_output = Command::new(&cli_path)
        .args(["inspect", "#mouse-test-card", "--pid", &pid.to_string()])
        .output()
        .expect("blitz-host inspect with selector failed");
    assert!(
        sel_inspect_output.status.success(),
        "blitz-host inspect #mouse-test-card must succeed"
    );
    let sel_inspect_stdout = String::from_utf8_lossy(&sel_inspect_output.stdout);
    let sel_inspect_json: serde_json::Value = serde_json::from_str(sel_inspect_stdout.trim())
        .expect("blitz-host inspect must output valid JSON");
    assert!(sel_inspect_json["nodeCount"].as_u64().unwrap_or(0) > 0);
    assert_eq!(sel_inspect_json["nodes"][0]["domId"], "mouse-test-card");

    // 14.6: CLI capture node via CSS selector
    let cli_sel_crop = "target/cli_proof_selector_card.png";
    let _ = std::fs::remove_file(cli_sel_crop);
    let sel_cap_output = Command::new(&cli_path)
        .args([
            "capture",
            "#mouse-test-card",
            "-o",
            cli_sel_crop,
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host capture with selector failed");
    assert!(
        sel_cap_output.status.success(),
        "blitz-host capture #mouse-test-card must succeed"
    );
    let sel_cap_stdout = String::from_utf8_lossy(&sel_cap_output.stdout);
    let sel_cap_json: serde_json::Value = serde_json::from_str(sel_cap_stdout.trim())
        .expect("blitz-host capture must output valid metadata JSON");
    assert_eq!(sel_cap_json["success"], true);
    assert!(
        sel_cap_json["filePath"]
            .as_str()
            .unwrap()
            .ends_with(cli_sel_crop)
    );
    assert!(sel_cap_json.get("dataBase64").is_none());
    assert!(
        std::path::Path::new(cli_sel_crop).exists(),
        "Output selector PNG file must exist on disk"
    );

    // 14.7: CLI mouse move via CSS selector
    let sel_move_output = Command::new(&cli_path)
        .args([
            "mouse",
            "move",
            "#mouse-test-card",
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host mouse move with selector failed");
    assert!(
        sel_move_output.status.success(),
        "blitz-host mouse move #mouse-test-card must succeed"
    );
    let sel_move_stdout = String::from_utf8_lossy(&sel_move_output.stdout);
    let sel_move_json: serde_json::Value = serde_json::from_str(sel_move_stdout.trim())
        .expect("blitz-host mouse move must output valid JSON");
    assert_eq!(sel_move_json["success"], true);

    // 14.8: Non-existent selector MUST fail cleanly with exit code 1
    let fail_sel_output = Command::new(&cli_path)
        .args([
            "mouse",
            "click",
            "#non-existent-element-xyz",
            "--pid",
            &pid.to_string(),
        ])
        .output()
        .expect("blitz-host mouse click failed to execute");
    assert_eq!(
        fail_sel_output.status.code(),
        Some(1),
        "click on non-existent selector must exit with code 1"
    );
    let fail_sel_stderr = String::from_utf8_lossy(&fail_sel_output.stderr);
    assert!(
        fail_sel_stderr.contains("not found in document")
            || fail_sel_stderr.contains("Node or selector"),
        "stderr must report element/selector not found: {fail_sel_stderr}"
    );

    println!(
        "  • CSS selector targeting verified: inspect, click, focus, set-value, move, capture, and clean failure for missing elements!"
    );

    // Terminate child process cleanly
    let _ = child.kill();
    let _ = child.wait();

    println!("=================================================================");
    println!("LIVE ATTACH, CLICK, FOCUS, SET_VALUE, SUBTREE CAPTURE, CLI PROOF PASSED 100%!");
    println!("=================================================================");
}
