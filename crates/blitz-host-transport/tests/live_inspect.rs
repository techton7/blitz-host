use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use blitz_host_protocol::InspectRequest;
use blitz_host_transport::DebugClient;

#[test]
fn test_live_native_runner_attach_and_inspect() {
    let runner_path = "/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/target/debug/oxidase-native-runner";
    if !std::path::Path::new(runner_path).exists() {
        eprintln!("oxidase-native-runner binary not found at {}", runner_path);
        return;
    }

    println!("Starting oxidase-native-runner in feature-enabled dev mode (no --debug-control flag)...");
    let mut child = Command::new(runner_path)
        .arg("--interactive")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn oxidase-native-runner");


    let pid = child.id();
    println!("Spawned child process with PID: {}", pid);

    // Poll for discovery and attach via PID targeting (wait up to 10 seconds for window mount & descriptor write)
    let mut client = None;
    for attempt in 1..=25 {
        thread::sleep(Duration::from_millis(400));
        if let Ok(c) = DebugClient::connect_pid(pid) {
            println!("Successfully connected to live host via PID {} on attempt #{}", pid, attempt);
            client = Some(c);
            break;
        }
    }

    let mut client = match client {
        Some(c) => c,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Failed to discover and connect to running oxidase-native-runner host within 10s");
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

    assert!(inspect_result.node_count > 0, "node count must be greater than zero");
    assert_eq!(inspect_result.nodes[0].tag, "#document");

    // Verify presence of real component tags
    let tags: Vec<&str> = inspect_result.nodes.iter().map(|n| n.tag.as_str()).collect();
    println!("  • Observed tags: {:?}", tags);

    let has_button = inspect_result
        .nodes
        .iter()
        .any(|n| n.tag == "button" || n.dom_id.as_deref() == Some("test-interaction-button"));
    assert!(has_button, "must have found button node in live component tree");

    let nodes_with_bounds = inspect_result.nodes.iter().filter(|n| n.bounds.is_some()).count();
    println!("  • Nodes with layout bounds: {}", nodes_with_bounds);
    assert!(nodes_with_bounds > 0, "at least some nodes must have layout bounds");

    // Helper to extract text under a node hierarchy (handles direct #text or nested anonymous blocks)
    fn extract_text_under(nodes: &[blitz_host_protocol::SemanticNode], root_id: u64) -> Option<String> {
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
    println!("Act Response: success={}, message={:?}", act_resp.success, act_resp.message);
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
    println!("  • Observed button text after click + settle(2): {:?}", updated_text);
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
    println!("  • Final button text after second click + settle_until: {:?}", final_text);
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
    println!("  • Found input node #{} (dom_id: {:?})", input_id, input_node.dom_id);
    assert_ne!(input_node.focused, Some(true), "input node must not be focused initially");

    // =========================================================================
    // STEP 6: Act (Dispatch Focus Action) & Settle
    // =========================================================================
    println!("Dispatching focus action to input node #{}...", input_id);
    let focus_resp = client.focus(input_id).expect("focus action failed");
    println!("Focus Response: success={}, message={:?}", focus_resp.success, focus_resp.message);
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

    println!("  • Observed focused_node_id: {:?}", post_focus_inspect.focused_node_id);
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
    println!("  • Conditional focus indicator rendered: {}", has_focus_indicator);
    assert!(
        has_focus_indicator,
        "CRITICAL PROOF: Dioxus must render focus indicator span in response to onfocus event"
    );

    // =========================================================================
    // STEP 7: Act (Dispatch SetValue Action) & Settle
    // =========================================================================
    let test_typed_str = "Hello from blitz-host live test!";
    println!("Dispatching set_value action to input node #{} with value {:?}...", input_id, test_typed_str);
    let set_val_resp = client.set_value(input_id, test_typed_str).expect("set_value action failed");
    println!("SetValue Response: success={}, message={:?}", set_val_resp.success, set_val_resp.message);
    assert!(set_val_resp.success, "set_value response must indicate success");

    println!("Sending settle request for 2 frames after set_value...");
    let settle_val = client.settle(2).expect("settle request after set_value failed");
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
    let set_val2_resp = client.set_value(input_id, second_test_str).expect("second set_value failed");
    assert!(set_val2_resp.success);

    let expected_p_text2 = format!("Typed: {second_test_str}");
    let settled_val_snapshot = client
        .settle_until(Duration::from_secs(5), |snap| {
            if let Some(p) = snap.nodes.iter().find(|n| n.dom_id.as_deref() == Some("typed-text")) {
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
    println!("  • Final typed text after settle_until: {:?}", final_typed_text);
    assert_eq!(final_typed_text.as_deref(), Some(expected_p_text2.as_str()));

    // Terminate child process cleanly
    let _ = child.kill();
    let _ = child.wait();

    println!("=================================================================");
    println!("LIVE ATTACH, CLICK, FOCUS, SET_VALUE, AND SETTLE PROOF PASSED 100%!");
    println!("=================================================================");
}
