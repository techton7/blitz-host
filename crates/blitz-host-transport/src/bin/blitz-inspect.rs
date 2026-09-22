//! # blitz-inspect
//!
//! CLI diagnostic, inspection, and action controller utility for live Blitz desktop windows.
//! Automatically discovers active local Blitz hosts and enables external inspection and event triggering.

use std::path::PathBuf;

use blitz_host_protocol::InspectRequest;
use blitz_host_transport::DebugClient;

fn print_help() {
    println!(
        r#"blitz-inspect: Out-of-process inspector & controller for live Blitz desktop applications.

USAGE:
    blitz-inspect [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [DESCRIPTOR_PATH]        Direct path to a host descriptor JSON file.
                             If omitted, automatically discovers the most recent active host.

OPTIONS:
    -h, --help               Print this help message and exit
    -c, --click <NODE_ID>    Dispatch a synthetic click action to the target node ID
    -s, --settle <FRAMES>    Wait for N VSync frames to settle (default: 2 if --click, else 0)
        --json               Output inspected semantic DOM tree in raw JSON format (for jq/agents)

EXAMPLES:
    # 1. Auto-discover active Blitz window and print DOM hierarchy
    blitz-inspect

    # 2. Output full semantic DOM snapshot as JSON for scripting/jq
    blitz-inspect --json

    # 3. Click target button #4294967402 and verify updated tree after 2 frames
    blitz-inspect --click 4294967402

    # 4. Connect to explicit host descriptor file
    blitz-inspect /tmp/blitz-host/15365-1790037052950147000.json
"#
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    let is_json = args.iter().any(|a| a == "--json");
    let explicit_path = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("-") && a.ends_with(".json"))
        .map(PathBuf::from);

    let mut client = match DebugClient::connect_discovered(explicit_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error connecting to Blitz host: {e}");
            eprintln!("Make sure a Blitz host is running with `--debug-control` active.");
            std::process::exit(1);
        }
    };

    let desc = client.descriptor().clone();

    // Check for optional action flag: -c / --click <node_id>
    let click_node_id = args
        .iter()
        .position(|a| a == "-c" || a == "--click")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|s| s.parse::<u64>().ok());

    let settle_frames = args
        .iter()
        .position(|a| a == "-s" || a == "--settle")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| if click_node_id.is_some() { Some(2) } else { None });

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
            }
        }
    }

    let response = client.inspect(InspectRequest::default())?;

    if is_json {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    println!("=================================================================");
    println!("[blitz-inspect] Blitz Host Live Inspector & Controller");
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
    println!("[blitz-inspect] Inspection completed successfully!");

    Ok(())
}
