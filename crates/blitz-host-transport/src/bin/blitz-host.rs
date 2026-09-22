//! # blitz-host
//!
//! Canonical CLI control plane and inspection tool for live Blitz desktop applications.
//! Provides out-of-process inspection and action dispatching.

use std::path::PathBuf;

use blitz_host_protocol::InspectRequest;
use blitz_host_transport::DebugClient;

fn print_main_help() {
    println!(
        r#"blitz-host: Out-of-process control plane for live Blitz desktop applications.

USAGE:
    blitz-host <SUBCOMMAND>

SUBCOMMANDS:
    inspect [OPTIONS]        Inspect live window semantic DOM & layout tree
    click <NODE_ID>          Dispatch a synthetic click action to target element (auto-settles)

OPTIONS:
    -h, --help               Print help information
    -V, --version            Print version information

Run 'blitz-host <SUBCOMMAND> --help' for more information on a specific subcommand.

EXAMPLES:
    blitz-host inspect
    blitz-host inspect --json
    blitz-host click 4294967402
"#
    );
}

fn print_inspect_help() {
    println!(
        r#"blitz-host-inspect: Inspect live Blitz window semantic DOM and layout tree.

USAGE:
    blitz-host inspect [OPTIONS] [DESCRIPTOR_PATH]

OPTIONS:
        --json               Output inspected tree in raw JSON format (for jq / AI agents)
    -h, --help               Print help information

ARGUMENTS:
    [DESCRIPTOR_PATH]        Path to host descriptor JSON file.
                             If omitted, auto-discovers the active running Blitz window.

EXAMPLES:
    # 1. Print formatted human-readable DOM tree with layout bounds
    blitz-host inspect

    # 2. Output full semantic DOM snapshot as JSON
    blitz-host inspect --json

    # 3. Connect to an explicit host descriptor file
    blitz-host inspect --json /tmp/blitz-host/15365-1790037052950147000.json
"#
    );
}

fn print_click_help() {
    println!(
        r#"blitz-host-click: Dispatch synthetic click to a live Blitz window element.

USAGE:
    blitz-host click [OPTIONS] <NODE_ID> [DESCRIPTOR_PATH]

ARGUMENTS:
    <NODE_ID>                Target node integer ID to click (e.g. 4294967402)
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching the click
    to ensure reactive state changes and layout recalculations have completed.

EXAMPLES:
    blitz-host click 4294967402
"#
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // If run with no arguments or top-level -h/--help:
    if args.len() <= 1 || (args.len() == 2 && (args[1] == "-h" || args[1] == "--help")) {
        print_main_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("blitz-host {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let subcmd = args[1].as_str();

    match subcmd {
        "inspect" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_inspect_help();
                return Ok(());
            }

            let is_json = subargs.iter().any(|a| a == "--json");
            let descriptor_path = subargs
                .iter()
                .find(|a| !a.starts_with('-') && a.ends_with(".json"))
                .map(PathBuf::from);

            let mut client = match DebugClient::connect_discovered(descriptor_path.as_deref()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error connecting to Blitz host: {e}");
                    eprintln!("Make sure a Blitz host is running with `--debug-control` active.");
                    std::process::exit(1);
                }
            };

            let desc = client.descriptor().clone();
            let response = client.inspect(InspectRequest::default())?;

            if is_json {
                println!("{}", serde_json::to_string_pretty(&response)?);
                return Ok(());
            }

            println!("=================================================================");
            println!("[blitz-host] Blitz Host Live Inspector");
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
            println!("[blitz-host] Inspection completed successfully!");
            Ok(())
        }
        "click" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_click_help();
                return Ok(());
            }

            let node_id_str = subargs.iter().find(|a| !a.starts_with('-') && !a.ends_with(".json"));
            let node_id: u64 = match node_id_str.and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => {
                    eprintln!("Error: 'click' requires a target <NODE_ID> argument.");
                    eprintln!("Run 'blitz-host click --help' for usage.");
                    std::process::exit(1);
                }
            };

            let descriptor_path = subargs
                .iter()
                .find(|a| !a.starts_with('-') && a.ends_with(".json"))
                .map(PathBuf::from);

            let mut client = match DebugClient::connect_discovered(descriptor_path.as_deref()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error connecting to Blitz host: {e}");
                    eprintln!("Make sure a Blitz host is running with `--debug-control` active.");
                    std::process::exit(1);
                }
            };

            println!("Dispatching click action to node #{} on live window...", node_id);
            let act_res = client.click(node_id)?;
            println!(
                "  • Act response: success={}, message={:?}",
                act_res.success, act_res.message
            );

            // Auto-settle 2 frames to ensure DOM and layout mutation settled
            println!("Synchronizing 2 VSync frames on live window...");
            let settle_res = client.settle(2)?;
            println!(
                "  • Settle response: settled={}, current_frame={}",
                settle_res.settled, settle_res.current_frame
            );
            println!("=================================================================");
            println!("[blitz-host] Click action completed and settled successfully!");
            Ok(())
        }
        unknown => {
            eprintln!("Error: Unknown subcommand '{}'.", unknown);
            eprintln!("Run 'blitz-host --help' for available subcommands.");
            std::process::exit(1);
        }
    }
}
