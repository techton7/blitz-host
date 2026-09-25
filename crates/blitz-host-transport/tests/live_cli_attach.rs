use std::process::Command;
use std::thread;

use blitz_host_protocol::{ControlRequest, ControlResponse, InspectResponse, SemanticNode};
use blitz_host_transport::DebugServer;

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

#[test]
fn test_live_cli_attach_and_inspect_named_pipe() {
    let (server, rx) = DebugServer::start("test-live-host", "0.1.0").expect("server must start");
    let descriptor = server.descriptor().clone();
    let pid = descriptor.pid;
    let socket_path = descriptor.socket_path.clone();

    println!("Spawned live DebugServer host with PID {pid} on socket {socket_path}");

    // Spawn server responder thread
    let server_thread = thread::spawn(move || {
        while let Ok(bridge_req) = rx.recv() {
            match &bridge_req.request {
                ControlRequest::Inspect(inspect_req) => {
                    println!("[HOST] Received inspect request: {:?}", inspect_req);
                    bridge_req.respond(ControlResponse::InspectSuccess(InspectResponse {
                        document_id: 1,
                        root_id: 100,
                        node_count: 2,
                        current_frame: Some(1),
                        focused_node_id: None,
                        hover_node_id: None,
                        viewport_scroll: None,
                        nodes: vec![
                            SemanticNode {
                                id: 100,
                                parent_id: None,
                                tag: "div".into(),
                                dom_id: Some("root".into()),
                                role: Some("generic".into()),
                                text: None,
                                bounds: Some([0.0, 0.0, 800.0, 600.0]),
                                focused: None,
                                hovered: None,
                                active: None,
                                scroll_offset: None,
                                children: vec![101],
                            },
                            SemanticNode {
                                id: 101,
                                parent_id: Some(100),
                                tag: "button".into(),
                                dom_id: Some("submit-btn".into()),
                                role: Some("button".into()),
                                text: Some("Submit".into()),
                                bounds: Some([50.0, 50.0, 120.0, 40.0]),
                                focused: None,
                                hovered: None,
                                active: None,
                                scroll_offset: None,
                                children: vec![],
                            },
                        ],
                        message: None,
                    }));
                }
                other => {
                    println!("[HOST] Unexpected request: {:?}", other);
                    bridge_req.respond(ControlResponse::Error("Unsupported in mock test".into()));
                    break;
                }
            }
        }
    });

    // Locate the blitz-host CLI executable
    let cli_path = build_blitz_host_cli();

    // 1. Test blitz-host list
    println!("Testing 'blitz-host list' CLI over IPC...");
    let list_output = Command::new(&cli_path)
        .arg("list")
        .output()
        .expect("blitz-host list failed");
    assert!(
        list_output.status.success(),
        "blitz-host list must exit with 0"
    );
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    println!("blitz-host list output:\n{}", list_stdout);
    let list_json: serde_json::Value =
        serde_json::from_str(list_stdout.trim()).expect("blitz-host list must output valid JSON");
    assert!(list_json.is_array());
    let found = list_json
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["pid"] == pid && h["socketPath"] == socket_path);
    assert!(
        found,
        "blitz-host list must discover our active host descriptor"
    );

    // 2. Test blitz-host inspect by PID
    println!(
        "Testing 'blitz-host inspect --pid {}' CLI over Named Pipe...",
        pid
    );
    let inspect_output = Command::new(&cli_path)
        .args(["inspect", "--pid", &pid.to_string()])
        .output()
        .expect("blitz-host inspect failed");
    assert!(
        inspect_output.status.success(),
        "blitz-host inspect must exit with 0"
    );
    let inspect_stdout = String::from_utf8_lossy(&inspect_output.stdout);
    println!("blitz-host inspect output:\n{}", inspect_stdout);
    let inspect_json: serde_json::Value = serde_json::from_str(inspect_stdout.trim())
        .expect("blitz-host inspect must output valid JSON");
    assert_eq!(inspect_json["documentId"], 1);
    assert_eq!(inspect_json["rootId"], 100);
    assert_eq!(inspect_json["nodeCount"], 2);
    assert_eq!(inspect_json["nodes"][0]["tag"], "div");
    assert_eq!(inspect_json["nodes"][1]["tag"], "button");
    assert_eq!(inspect_json["nodes"][1]["domId"], "submit-btn");

    // 3. Test blitz-host inspect by explicit selector (Auto discovery since single host)
    println!("Testing 'blitz-host inspect' auto-discovering single host...");
    let auto_inspect_output = Command::new(&cli_path)
        .arg("inspect")
        .output()
        .expect("blitz-host auto inspect failed");
    assert!(auto_inspect_output.status.success());
    let auto_json: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&auto_inspect_output.stdout).trim())
            .expect("auto inspect must output valid JSON");
    assert_eq!(auto_json["documentId"], 1);

    // 4. Test blitz-host inspect by EXPLICIT Named Pipe endpoint via CLI
    println!(
        "Testing 'blitz-host inspect {}' CLI over explicit Named Pipe path...",
        socket_path
    );
    let explicit_cli_output = Command::new(&cli_path)
        .args(["inspect", &socket_path])
        .output()
        .expect("blitz-host explicit pipe inspect failed");
    assert!(
        explicit_cli_output.status.success(),
        "blitz-host inspect with explicit pipe must exit with 0"
    );
    let explicit_cli_json: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&explicit_cli_output.stdout).trim())
            .expect("explicit CLI inspect must output valid JSON");
    assert_eq!(explicit_cli_json["documentId"], 1);
    assert_eq!(explicit_cli_json["rootId"], 100);

    // 5. Test programmatic DebugClient::connect_target with TargetSelector::ExplicitPath
    println!(
        "Testing programmatic DebugClient::connect_target with TargetSelector::ExplicitPath..."
    );
    let mut explicit_client = blitz_host_transport::DebugClient::connect_target(
        &blitz_host_transport::TargetSelector::ExplicitPath(std::path::PathBuf::from(&socket_path)),
    )
    .expect("programmatic explicit path connection must succeed");
    let explicit_resp = explicit_client
        .inspect(blitz_host_protocol::InspectRequest::default())
        .expect("inspect over explicit client must succeed");
    assert_eq!(explicit_resp.document_id, 1);
    assert_eq!(explicit_resp.node_count, 2);
    drop(explicit_client);

    // 6. Test that an unreachable explicit pipe path fails explicitly with ConnectionRefused
    println!("Testing unreachable explicit Named Pipe path rejection...");
    let dead_pipe_path = r"\\.\pipe\blitz-host-nonexistent-endpoint-test";
    let unreachable_res = blitz_host_transport::DebugClient::connect_target(
        &blitz_host_transport::TargetSelector::ExplicitPath(std::path::PathBuf::from(
            dead_pipe_path,
        )),
    );
    assert!(
        unreachable_res.is_err(),
        "unreachable Named Pipe must return error"
    );
    let err = unreachable_res.err().unwrap();
    assert_eq!(
        err.kind(),
        std::io::ErrorKind::ConnectionRefused,
        "error kind must be ConnectionRefused"
    );
    println!("Unreachable pipe correctly rejected: {err}");

    // 7. Test CLI against unreachable explicit Named Pipe path exits with code 1
    let dead_cli_output = Command::new(&cli_path)
        .args(["inspect", dead_pipe_path])
        .output()
        .expect("blitz-host inspect against dead pipe must execute");
    assert_eq!(
        dead_cli_output.status.code(),
        Some(1),
        "CLI must exit with code 1 for unreachable pipe"
    );
    let dead_stderr = String::from_utf8_lossy(&dead_cli_output.stderr);
    assert!(
        dead_stderr.contains("unreachable") || dead_stderr.contains("Error connecting"),
        "stderr must report connection error"
    );

    // Shutdown server
    server.shutdown();
    let _ = server_thread.join();
    println!("Test completed successfully!");
}
