//! # blitz-host
//!
//! Canonical CLI control plane and inspection tool for live Blitz desktop applications.
//! Provides out-of-process inspection, process listing, and action dispatching.

use std::path::PathBuf;

use blitz_host::client::{DebugClient, TargetSelector};
use blitz_host::protocol::{InspectRequest, KeyModifiers};

fn print_main_help() {
    println!(
        r#"blitz-host: Out-of-process control plane for live Blitz desktop applications.

USAGE:
    blitz-host <SUBCOMMAND>

SUBCOMMANDS:
    list [OPTIONS]                 List active, reachable Blitz desktop host processes
    inspect [OPTIONS]              Inspect live window semantic DOM & layout tree
    capture [OPTIONS]              Capture live rendered visual screenshot (PNG)
    click <NODE_ID> [OPTIONS]       Dispatch a synthetic click action to target element (auto-settles)
    focus <NODE_ID> [OPTIONS]       Focus target element (auto-settles)
    set-value <NODE_ID> <VALUE>    Set text value of an input element (auto-settles)
    key <KEY> [OPTIONS]            Dispatch a synthetic key event (auto-settles)

OPTIONS:
    -h, --help                     Print help information
    -V, --version                  Print version information

Run 'blitz-host <SUBCOMMAND> --help' for more information on a specific subcommand.

EXAMPLES:
    # List active hosts
    blitz-host list

    # Inspect the default or targeted host
    blitz-host inspect
    blitz-host inspect --pid 37462
    blitz-host inspect --json

    # Capture visual screenshot (PNG)
    blitz-host capture
    blitz-host capture --pid 37462
    blitz-host capture -o screenshot.png --pid 37462

    # Click a node
    blitz-host click 4294967402
    blitz-host click 4294967402 --pid 37462

    # Focus an input node
    blitz-host focus 4294967405 --pid 37462

    # Set value on an input node
    blitz-host set-value 4294967405 "Hello Blitz" --pid 37462
"#
    );
}

fn print_capture_help() {
    println!(
        r#"blitz-host-capture: Capture live rendered visual screenshot (PNG).

USAGE:
    blitz-host capture [OPTIONS] [DESCRIPTOR_PATH]

OPTIONS:
        --pid <PID>          Target specific host process by OS process ID
    -o, --output <PATH>      Output PNG file path (defaults to blitz-capture-<PID>.png)
        --json               Output capture response as JSON with base64 data
    -h, --help               Print help information

ARGUMENTS:
    [DESCRIPTOR_PATH]        Path to host descriptor JSON file or UDS socket.
                             If omitted, auto-discovers the active running Blitz window.

EXAMPLES:
    # 1. Capture visual screenshot and save to default file (blitz-capture-<PID>.png)
    blitz-host capture

    # 2. Capture and save to explicit file path
    blitz-host capture -o screenshot.png --pid 37462

    # 3. Output capture response as JSON
    blitz-host capture --json --pid 37462
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

fn print_focus_help() {
    println!(
        r#"blitz-host-focus: Focus a target node in a live Blitz window.

USAGE:
    blitz-host focus <NODE_ID> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <NODE_ID>                Target node integer ID to focus (e.g. 4294967405)
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching focus
    to ensure focus styling and event propagation have completed.

EXAMPLES:
    blitz-host focus 4294967405
    blitz-host focus 4294967405 --pid 37462
"#
    );
}

fn print_set_value_help() {
    println!(
        r#"blitz-host-set-value: Set text value on an input element in a live Blitz window.

USAGE:
    blitz-host set-value <NODE_ID> <VALUE> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <NODE_ID>                Target node integer ID of input element (e.g. 4294967405)
    <VALUE>                  Text string to inject into the input element
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after setting the value
    to ensure reactive signal updates and layout recalculations have completed.

EXAMPLES:
    blitz-host set-value 4294967405 "Hello Blitz"
    blitz-host set-value 4294967405 "Hello Blitz" --pid 37462
"#
    );
}

fn print_key_help() {
    println!(
        r#"blitz-host-key: Dispatch a synthetic key event to a live Blitz window.

USAGE:
    blitz-host key <KEY> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <KEY>                    Key name or compound specifier (e.g. Tab, Shift+Tab, Enter, Space, Escape, Backspace, Delete, ArrowLeft, ArrowRight, ArrowUp, ArrowDown, a, z)
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
        --node <NODE_ID>     Target specific node integer ID (e.g. 4294967405). If omitted, dispatches to currently focused element.
        --shift              Hold Shift modifier
        --ctrl               Hold Ctrl modifier
        --alt                Hold Alt/Option modifier
        --meta               Hold Meta/Cmd modifier
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching the key
    to ensure reactive updates and layout recalculations have completed.

EXAMPLES:
    # 1. Focus traversal
    blitz-host key Tab
    blitz-host key Tab --shift
    blitz-host key Shift+Tab

    # 2. Focus clearing
    blitz-host key Escape

    # 3. Activation
    blitz-host key Enter
    blitz-host key Space

    # 4. Text editing
    blitz-host key a --node 4294967405
    blitz-host key Backspace --node 4294967405

    # 5. Select all
    blitz-host key a --meta --node 4294967405
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

fn parse_node_arg(args: &[String]) -> Option<u64> {
    for i in 0..args.len() {
        if args[i] == "--node" && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
        if let Some(rest) = args[i].strip_prefix("--node=") {
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

fn parse_output_arg(args: &[String]) -> Option<PathBuf> {
    for i in 0..args.len() {
        if (args[i] == "-o" || args[i] == "--output") && i + 1 < args.len() {
            return Some(PathBuf::from(&args[i + 1]));
        }
        if let Some(rest) = args[i].strip_prefix("--output=") {
            return Some(PathBuf::from(rest));
        }
        if let Some(rest) = args[i].strip_prefix("-o=") {
            return Some(PathBuf::from(rest));
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

                let focus_str = if node.focused == Some(true) { " [FOCUSED]" } else { "" };

                println!(
                    "{}{} {}{}{}{}{}{}",
                    indent, id_str, tag_str, dom_id_str, role_str, text_str, bounds_str, focus_str
                );
            }

            println!("=================================================================");
            println!("[blitz-host] Inspection completed successfully!");
            Ok(())
        }
        "capture" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_capture_help();
                return Ok(());
            }

            let is_json = subargs.iter().any(|a| a == "--json");
            let output_path = parse_output_arg(subargs);
            let selector = determine_selector(subargs);

            let mut client = match DebugClient::connect_target(&selector) {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("Error connecting to Blitz host: {err}");
                    eprintln!("Make sure a Blitz host is running with `blitz-host` enabled.");
                    eprintln!("Use 'blitz-host list' to inspect available hosts.");
                    std::process::exit(1);
                }
            };

            let pid = client.descriptor().pid;
            let resp = match client.capture() {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("Error capturing visual screenshot: {err}");
                    std::process::exit(1);
                }
            };

            if is_json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
                return Ok(());
            }

            if !resp.success {
                eprintln!(
                    "Capture failed: {}",
                    resp.message.as_deref().unwrap_or("unknown error")
                );
                std::process::exit(1);
            }

            use base64::prelude::*;
            let png_bytes = match BASE64_STANDARD.decode(&resp.data_base64) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Error decoding PNG base64 payload: {e}");
                    std::process::exit(1);
                }
            };

            let target_file = output_path.unwrap_or_else(|| {
                PathBuf::from(format!("blitz-capture-{pid}.png"))
            });

            if let Some(parent) = target_file.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&target_file, &png_bytes)?;

            println!("=================================================================");
            println!("[blitz-host] Visual Screenshot Captured");
            println!("=================================================================");
            println!("  • Process PID : {}", pid);
            println!("  • Resolution  : {}x{} physical pixels", resp.width, resp.height);
            println!("  • Format      : {}", resp.format.to_uppercase());
            println!("  • Size        : {} bytes", png_bytes.len());
            println!("  • Saved To    : {}", target_file.display());
            println!("=================================================================");
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
        "focus" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_focus_help();
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
                    eprintln!("Error: 'focus' requires a target <NODE_ID> argument.");
                    eprintln!("Run 'blitz-host focus --help' for usage.");
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
                "Dispatching focus action to node #{} (PID: {})...",
                node_id,
                client.descriptor().pid
            );
            let act_res = client.focus_window(window_id, node_id)?;
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
            println!("[blitz-host] Focus action completed and settled successfully!");
            Ok(())
        }
        "set-value" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_set_value_help();
                return Ok(());
            }

            // Extract positional non-flag arguments (skip flags and flag arguments)
            let mut positional = Vec::new();
            let mut skip_next = false;
            for arg in subargs {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "--pid" || arg == "--window" || arg == "--window-id" {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with("--pid=")
                    || arg.starts_with("--window=")
                    || arg.starts_with("--window-id=")
                    || arg.starts_with('-')
                {
                    continue;
                }
                if arg.ends_with(".json") || arg.ends_with(".sock") {
                    continue;
                }
                positional.push(arg.as_str());
            }

            if positional.is_empty() {
                eprintln!("Error: 'set-value' requires target <NODE_ID> and <VALUE> arguments.");
                eprintln!("Run 'blitz-host set-value --help' for usage.");
                std::process::exit(1);
            }

            let node_id: u64 = match positional[0].parse() {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("Error: Invalid <NODE_ID> '{}'. Must be an integer.", positional[0]);
                    std::process::exit(1);
                }
            };

            let value = if positional.len() > 1 {
                positional[1].to_string()
            } else {
                String::new()
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
                "Dispatching set-value action (value: {:?}) to node #{} (PID: {})...",
                value,
                node_id,
                client.descriptor().pid
            );
            let act_res = client.set_value_window(window_id, node_id, value)?;
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
            println!("[blitz-host] Set-value action completed and settled successfully!");
            Ok(())
        }
        "key" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_key_help();
                return Ok(());
            }

            // Extract positional non-flag arguments
            let mut positional = Vec::new();
            let mut skip_next = false;
            for arg in subargs {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "--pid" || arg == "--window" || arg == "--window-id" || arg == "--node" {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with("--pid=")
                    || arg.starts_with("--window=")
                    || arg.starts_with("--window-id=")
                    || arg.starts_with("--node=")
                    || arg.starts_with('-')
                {
                    continue;
                }
                if arg.ends_with(".json") || arg.ends_with(".sock") {
                    continue;
                }
                positional.push(arg.as_str());
            }

            if positional.is_empty() {
                eprintln!("Error: 'key' requires a <KEY> argument (e.g. Tab, Shift+Tab, Enter, Space, Escape, Backspace).");
                eprintln!("Run 'blitz-host key --help' for usage.");
                std::process::exit(1);
            }

            let key_str = positional[0];
            let node_id = parse_node_arg(subargs);
            let window_id = parse_window_arg(subargs);
            let shift = subargs.iter().any(|a| a == "--shift");
            let ctrl = subargs.iter().any(|a| a == "--ctrl");
            let alt = subargs.iter().any(|a| a == "--alt");
            let meta = subargs.iter().any(|a| a == "--meta" || a == "--cmd");
            let modifiers = if shift || ctrl || alt || meta {
                Some(KeyModifiers {
                    shift,
                    ctrl,
                    alt,
                    meta,
                })
            } else {
                None
            };

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
                "Dispatching key action '{key_str}' (node: {:?}, modifiers: {:?}) (PID: {})...",
                node_id,
                modifiers,
                client.descriptor().pid
            );
            let act_res = client.key_with_modifiers(window_id, node_id, key_str, modifiers)?;
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
            println!("[blitz-host] Key action completed and settled successfully!");
            Ok(())
        }
        unknown => {
            eprintln!("Error: Unknown subcommand '{}'.", unknown);
            eprintln!("Run 'blitz-host --help' for available subcommands.");
            std::process::exit(1);
        }
    }
}
