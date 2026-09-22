//! # blitz-host
//!
//! Canonical CLI control plane and inspection tool for live Blitz desktop applications.
//! Provides out-of-process inspection, typed action dispatching, and frame settlement.

use std::path::PathBuf;

use blitz_host_protocol::InspectRequest;
use blitz_host_transport::DebugClient;

fn print_help() {
    println!(
        r#"blitz-host: Out-of-process control plane and live DOM inspector for Blitz applications.

USAGE:
    blitz-host [SUBCOMMAND | OPTIONS] [DESCRIPTOR_PATH]

SUBCOMMANDS:
    inspect                  Inspect and display the live semantic DOM tree (default)
    click <NODE_ID>          Dispatch a synthetic click action to the target node ID
    settle <FRAMES>          Wait for N VSync frames to settle

OPTIONS:
    -h, --help               Print this help message and exit
    -V, --version            Print version information
    -c, --click <NODE_ID>    (Flag mode) Dispatch click to target node ID
    -s, --settle <FRAMES>    (Flag mode) Wait for N VSync frames (default: 2 if clicking)
        --json               Output response in raw JSON format (for jq / AI agents)

ARGUMENTS:
    [DESCRIPTOR_PATH]        Path to host descriptor JSON file.
                             If omitted, auto-discovers the active running Blitz window.

EXAMPLES:
    # 1. Inspect live Blitz window (default)
    blitz-host

    # 2. Output live semantic DOM tree as JSON
    blitz-host --json
    blitz-host inspect --json

    # 3. Click target button #4294967402 on the live window
    blitz-host click 4294967402
    blitz-host --click 4294967402

    # 4. Wait for 5 VSync frames
    blitz-host settle 5
"#
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("blitz-host {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let is_json = args.iter().any(|a| a == "--json");

    // Filter positional args (skipping binary name and flags)
    let non_flags: Vec<&str> = args
        .iter()
        .skip(1)
        .map(|s| s.as_str())
        .filter(|s| !s.starts_with('-'))
        .collect();

    // Parse subcommands or flags
    let mut click_node_id: Option<u64> = None;
    let mut settle_frames: Option<u32> = None;
    let mut descriptor_path: Option<PathBuf> = None;

    // Check subcommand style: `blitz-host click 1234`
    if let Some(&subcmd) = non_flags.first() {
        match subcmd {
            "click" => {
                if let Some(id_str) = non_flags.get(1) {
                    click_node_id = id_str.parse().ok();
                }
                if let Some(path_str) = non_flags.get(2) {
                    descriptor_path = Some(PathBuf::from(path_str));
                }
            }
            "settle" => {
                if let Some(frame_str) = non_flags.get(1) {
                    settle_frames = frame_str.parse().ok();
                }
                if let Some(path_str) = non_flags.get(2) {
                    descriptor_path = Some(PathBuf::from(path_str));
                }
            }
            "inspect" => {
                if let Some(path_str) = non_flags.get(1) {
                    descriptor_path = Some(PathBuf::from(path_str));
                }
            }
            other => {
                if other.ends_with(".json") {
                    descriptor_path = Some(PathBuf::from(other));
                }
            }
        }
    }

    // Check flag style overrides: `-c / --click <id>`, `-s / --settle <n>`
    if let Some(pos) = args.iter().position(|a| a == "-c" || a == "--click") {
        if let Some(id_str) = args.get(pos + 1) {
            click_node_id = id_str.parse().ok();
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "-s" || a == "--settle") {
        if let Some(frame_str) = args.get(pos + 1) {
            settle_frames = frame_str.parse().ok();
        }
    }

    // Default settle for click is 2 frames
    if click_node_id.is_some() && settle_frames.is_none() {
        settle_frames = Some(2);
    }

    // Connect to host
    let mut client = match DebugClient::connect_discovered(descriptor_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error connecting to Blitz host: {e}");
            eprintln!("Make sure a Blitz host is running with `--debug-control` active.");
            std::process::exit(1);
        }
    };

    let desc = client.descriptor().clone();

    // Execute click action if requested
    if let Some(node_id) = click_node_id {
        if !is_json {
            println!("Dispatching click action to node #{} on live window...", node_id);
        }
        let act_res = client.click(node_id)?;
        if !is_json {
            println!(
                "  • Act response: success={}, message={:?}",
                act_res.success, act_res.message
            );
        }
    }

    // Execute settle if requested
    if let Some(frames) = settle_frames {
        if frames > 0 {
            if !is_json {
                println!("Settling {} frame(s) on live window...", frames);
            }
            let settle_res = client.settle(frames)?;
            if !is_json {
                println!(
                    "  • Settle response: settled={}, current_frame={}",
                    settle_res.settled, settle_res.current_frame
                );
                println!("-----------------------------------------------------------------");
            }
        }
    }

    // Perform inspect
    let response = client.inspect(InspectRequest::default())?;

    if is_json {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    println!("=================================================================");
    println!("[blitz-host] Blitz Host Live Inspector & Controller");
    println!("=================================================================");
    println!("Connected to host:");
    println!("  • Renderer        : {} v{}", desc.renderer, desc.renderer_version);
    println!("  • PID             : {}", desc.pid);
    println!("  • Instance ID     : {}", desc.instance_id);
    println!("  • Socket Path     : {}", desc.socket_path);
    println!("  • Protocol Version: {}", desc.protocol_version);
    println!("-----------------------------------------------------------------");
    println!("Received typed InspectResponse:");
    println!("  • Document ID: {}", response.document_id);
    println!("  • Root ID    : {}", response.root_id);
    println!("  • Node Count : {}", response.node_count);
    println!("-----------------------------------------------------------------");
    println!("Hierarchy:");

    for node in &response.nodes {
        let indent = "  ".repeat(if node.id == response.root_id { 0 } else { 1 });
        let id_str = format!("#{}", node.id);
        let tag_str = format!("<{}>", node.tag);
        let dom_id_str = node
            .dom_id
            .as_deref()
            .map(|id| format!(" id=\"{}\"", id))
            .unwrap_or_default();
        let role_str = node
            .role
            .as_deref()
            .map(|r| format!(" [role=\"{}\"]", r))
            .unwrap_or_default();
        let text_str = node
            .text
            .as_deref()
            .map(|t| format!(" \"{}\"", t.trim()))
            .unwrap_or_default();
        let bounds_str = node
            .bounds
            .map(|b| {
                format!(
                    " rect({:.1}, {:.1}, {:.1}, {:.1})",
                    b[0], b[1], b[2], b[3]
                )
            })
            .unwrap_or_default();

        println!(
            "{}{} {}{}{}{}{}",
            indent, id_str, tag_str, dom_id_str, role_str, text_str, bounds_str
        );
    }

    println!("=================================================================");
    println!("[blitz-host] Operation completed successfully!");

    Ok(())
}
