//! # blitz-host
//!
//! Canonical CLI control plane and inspection tool for live Blitz desktop applications.
//! Provides out-of-process inspection, process listing, and action dispatching.

use std::path::PathBuf;

use blitz_host::client::{DebugClient, TargetSelector};
use blitz_host::protocol::InspectRequest;

fn print_main_help() {
    println!(
        r#"blitz-host: Out-of-process control plane for live Blitz desktop applications.

USAGE:
    blitz-host <SUBCOMMAND>

SUBCOMMANDS:
    list [OPTIONS]           List active, reachable Blitz desktop host processes
    inspect [OPTIONS]        Inspect live window semantic DOM & layout tree
    click <NODE_ID> [OPTIONS] Dispatch a synthetic click action to target element (auto-settles)

OPTIONS:
    -h, --help               Print help information
    -V, --version            Print version information

Run 'blitz-host <SUBCOMMAND> --help' for more information on a specific subcommand.

EXAMPLES:
    # List active hosts
    blitz-host list

    # Inspect the default or targeted host
    blitz-host inspect
    blitz-host inspect --pid 37462
    blitz-host inspect --json

    # Click a node
    blitz-host click 4294967402
    blitz-host click 4294967402 --pid 37462
"#
    );
}

fn print_list_help() {
    println!(
        r#"blitz-host-list: List active, reachable Blitz desktop host processes.

USAGE:
    blitz-host list [OPTIONS]

OPTIONS:
        --json               Output list in raw JSON format (for jq / AI agents)
    -h, --help               Print help information

EXAMPLES:
    # 1. Print formatted human-readable table of running hosts
    blitz-host list

    # 2. Output running hosts as JSON
    blitz-host list --json
"#
    );
}

fn print_inspect_help() {
    println!(
        r#"blitz-host-inspect: Inspect live Blitz window semantic DOM and layout tree.

USAGE:
    blitz-host inspect [OPTIONS] [DESCRIPTOR_PATH]

OPTIONS:
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
        --json               Output inspected tree in raw JSON format (for jq / AI agents)
    -h, --help               Print help information

ARGUMENTS:
    [DESCRIPTOR_PATH]        Path to host descriptor JSON file or UDS socket.
                             If omitted, auto-discovers the active running Blitz window.

EXAMPLES:
    # 1. Print formatted human-readable DOM tree with layout bounds
    blitz-host inspect

    # 2. Target a specific process ID
    blitz-host inspect --pid 37462

    # 3. Output full semantic DOM snapshot as JSON
    blitz-host inspect --json

    # 4. Connect to an explicit host descriptor file
    blitz-host inspect --json /tmp/blitz-host/15365-1790037052950147000.json
"#
    );
}

fn print_click_help() {
    println!(
        r#"blitz-host-click: Dispatch synthetic click to a live Blitz window element.

USAGE:
    blitz-host click <NODE_ID> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <NODE_ID>                Target node integer ID to click (e.g. 4294967402)
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching the click
    to ensure reactive state changes and layout recalculations have completed.

EXAMPLES:
    blitz-host click 4294967402
    blitz-host click 4294967402 --pid 37462
"#
    );
}

fn parse_pid_arg(args: &[String]) -> Option<u32> {
    for i in 0..args.len() {
        if args[i] == "--pid" && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
        if let Some(rest) = args[i].strip_prefix("--pid=") {
            return rest.parse().ok();
        }
    }
    None
}

fn parse_window_arg(args: &[String]) -> Option<u64> {
    for i in 0..args.len() {
        if (args[i] == "--window" || args[i] == "--window-id") && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
        if let Some(rest) = args[i].strip_prefix("--window=") {
            return rest.parse().ok();
        }
        if let Some(rest) = args[i].strip_prefix("--window-id=") {
            return rest.parse().ok();
        }
    }
    None
}

fn determine_selector(args: &[String]) -> TargetSelector {
    if let Some(pid) = parse_pid_arg(args) {
        return TargetSelector::Pid(pid);
    }
    let explicit_path = args
        .iter()
        .find(|a| !a.starts_with('-') && (a.ends_with(".json") || a.ends_with(".sock")))
        .map(PathBuf::from);

    if let Some(path) = explicit_path {
        TargetSelector::ExplicitPath(path)
    } else {
        TargetSelector::Auto
    }
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
        "list" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_list_help();
                return Ok(());
            }

            let is_json = subargs.iter().any(|a| a == "--json");
            let hosts = blitz_host::transport::list_hosts()?;

            if is_json {
                println!("{}", serde_json::to_string_pretty(&hosts)?);
                return Ok(());
            }

            if hosts.is_empty() {
                println!("No active, reachable Blitz hosts found in $TMPDIR/blitz-host.");
                return Ok(());
            }

            println!("===================================================================================================");
            println!("[blitz-host] Active Blitz Host Processes ({})", hosts.len());
            println!("===================================================================================================");
            println!("{:<8} {:<20} {:<10} {:<12} {:<10} {:<30}", "PID", "RENDERER", "DOC ID", "WINDOW ID", "STATUS", "SOCKET");
            println!("{:-<8} {:-<20} {:-<10} {:-<12} {:-<10} {:-<30}", "", "", "", "", "", "");

            for h in &hosts {
                let renderer_str = format!("{} v{}", h.renderer, h.renderer_version);
                let doc_id_str = h
                    .primary_document_id
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let win_id_str = h
                    .primary_window_id
                    .map(|w| w.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let short_socket = if h.socket_path.len() > 30 {
                    format!("...{}", &h.socket_path[h.socket_path.len() - 27..])
                } else {
                    h.socket_path.clone()
                };

                println!(
                    "{:<8} {:<20} {:<10} {:<12} {:<10} {:<30}",
                    h.pid, renderer_str, doc_id_str, win_id_str, "reachable", short_socket
                );
            }
            println!("===================================================================================================");
            println!("Tip: Target a specific host with: blitz-host inspect --pid <PID>");
            Ok(())
        }
        "inspect" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_inspect_help();
                return Ok(());
            }

            let is_json = subargs.iter().any(|a| a == "--json");
            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            let mut client = match DebugClient::connect_target(&selector) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error connecting to Blitz host: {e}");
                    eprintln!("Make sure a Blitz host is running with `blitz-host` enabled.");
                    eprintln!("Use 'blitz-host list' to inspect available hosts.");
                    std::process::exit(1);
                }
            };

            let desc = client.descriptor().clone();
            let mut req = InspectRequest::default();
            req.window_id = window_id;
            let response = client.inspect(req)?;

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
            if let Some(win_id) = desc.primary_window_id {
                println!("  • Primary Window  : {}", win_id);
            }
            if let Some(doc_id) = desc.primary_document_id {
                println!("  • Primary Document: {}", doc_id);
            }
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

            let node_id_str = subargs.iter().find(|a| {
                !a.starts_with('-')
                    && !a.ends_with(".json")
                    && !a.ends_with(".sock")
                    && a.parse::<u64>().is_ok()
            });
            let node_id: u64 = match node_id_str.and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => {
                    eprintln!("Error: 'click' requires a target <NODE_ID> argument.");
                    eprintln!("Run 'blitz-host click --help' for usage.");
                    std::process::exit(1);
                }
            };

            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            let mut client = match DebugClient::connect_target(&selector) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error connecting to Blitz host: {e}");
                    eprintln!("Make sure a Blitz host is running with `blitz-host` enabled.");
                    eprintln!("Use 'blitz-host list' to inspect available hosts.");
                    std::process::exit(1);
                }
            };

            println!(
                "Dispatching click action to node #{} (PID: {})...",
                node_id,
                client.descriptor().pid
            );
            let act_res = client.click_window(window_id, node_id)?;
            println!(
                "  • Act response: success={}, message={:?}",
                act_res.success, act_res.message
            );

            // Auto-settle 2 frames to ensure DOM and layout mutation settled
            println!("Synchronizing 2 VSync frames on live window...");
            let settle_res = client.settle_window(window_id, 2)?;
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
