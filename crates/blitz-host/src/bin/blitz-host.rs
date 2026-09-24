//! # blitz-host
//!
//! Canonical CLI control plane and inspection tool for live Blitz desktop applications.
//! Provides out-of-process inspection, process listing, and action dispatching.

use std::path::PathBuf;

use blitz_host::client::{DebugClient, TargetSelector};
use blitz_host::protocol::{ElementTarget, InspectRequest, KeyModifiers};

fn print_main_help() {
    println!(
        r##"blitz-host: Out-of-process control plane for live Blitz desktop applications.

USAGE:
    blitz-host <SUBCOMMAND>

SUBCOMMANDS:
    list [OPTIONS]                 List active, reachable Blitz desktop host processes (JSON)
    inspect [TARGET] [OPTIONS]     Inspect live window semantic DOM & layout tree (JSON)
    capture [TARGET] -o <FILE>     Capture live visual screenshot or node crop (mandatory -o, JSON)
    focus <TARGET> [OPTIONS]       Focus target element (auto-settles, JSON)
    set-value <TARGET> <VALUE>     Set text value of an input element (auto-settles, JSON)
    key <KEY_SPEC> [OPTIONS]       Dispatch key event (e.g. cmd+a, shift+tab, enter) (JSON)
    mouse <SUBCOMMAND> [OPTIONS]   Pointer and mouse interactions (click, move, down, up, wheel, drag) (JSON)

ARGUMENTS:
    [TARGET]                       Target element specified by numeric node ID or CSS selector
                                   (e.g. 4294967402, "#test-input", "button.primary")

OPTIONS:
    -h, --help                     Print help information
    -V, --version                  Print version information

Run 'blitz-host <SUBCOMMAND> --help' for more information on a specific subcommand.

EXAMPLES:
    # 1. List active host processes (always JSON)
    blitz-host list

    # 2. Inspect the live DOM and layout tree (always JSON)
    blitz-host inspect
    blitz-host inspect 4294967464
    blitz-host inspect "#test-input"
    blitz-host inspect --pid 37462

    # 3. Capture visual screenshot to required output path
    blitz-host capture -o target/screenshot.png
    blitz-host capture 4294967464 -o target/card.png
    blitz-host capture "#mouse-test-card" -o target/card.png

    # 4. Focus, and set value using selectors or IDs
    blitz-host focus "#test-input"
    blitz-host set-value "#test-input" "Hello Blitz"

    # 5. Keyboard shortcuts with compound expressions
    blitz-host key enter
    blitz-host key cmd+a
    blitz-host key command+shift+z

    # 6. Mouse interactions via mouse namespace using selectors or IDs
    blitz-host mouse click "#test-interaction-button"
    blitz-host mouse move "#mouse-test-card"
    blitz-host mouse wheel "#test-scroll-container" --dy 50
    blitz-host mouse drag "#mouse-test-card" "#test-input"
"##
    );
}

fn print_capture_help() {
    println!(
        r##"blitz-host-capture: Capture live rendered visual screenshot (PNG).

USAGE:
    blitz-host capture [TARGET] -o <PATH> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]             Optional target element specified by numeric node ID or CSS selector
                         to crop capture to its visual bounds (e.g. 4294967464, "#mouse-test-card").
    [DESCRIPTOR_PATH]    Path to host descriptor JSON file or UDS socket.
                         If omitted, auto-discovers the active running Blitz window.

OPTIONS:
    -o, --output <PATH>  Output PNG file path (REQUIRED)
    --node, -n <NODE_ID> Target specific node/subtree by ID to crop capture
    --selector, -s <SEL> Target specific node/subtree by CSS selector to crop capture
        --pid <PID>      Target specific host process by OS process ID
        --window <ID>    Target specific window ID (optional, defaults to primary window)
    -h, --help           Print help information

OUTPUT:
    Always returns compact metadata JSON on stdout (no inline base64 blobs):
    {{
      "success": true,
      "filePath": "target/card.png",
      "width": 618,
      "height": 40,
      "format": "png",
      "nodeId": 4294967464,
      "bytes": 7443
    }}

EXAMPLES:
    # 1. Capture full window screenshot to specified file
    blitz-host capture -o target/full_window.png

    # 2. Capture specific element/subtree cropped to its bounds via ID or selector
    blitz-host capture 4294967464 -o target/card.png
    blitz-host capture "#mouse-test-card" -o target/card.png
    blitz-host capture --selector "#mouse-test-card" -o target/card.png

    # 3. Target specific process ID
    blitz-host capture "#mouse-test-card" -o target/card.png --pid 37462
"##
    );
}

fn print_list_help() {
    println!(
        r##"blitz-host-list: List active, reachable Blitz desktop host processes.

USAGE:
    blitz-host list [OPTIONS]

OPTIONS:
    -h, --help               Print help information

OUTPUT:
    Always outputs list in raw JSON array format on stdout.

EXAMPLES:
    blitz-host list
"##
    );
}

fn print_inspect_help() {
    println!(
        r##"blitz-host-inspect: Inspect live Blitz window semantic DOM and layout tree.

USAGE:
    blitz-host inspect [TARGET] [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]                 Optional root element specified by numeric node ID or CSS selector
                             to inspect only a specific subtree (e.g. 4294967464, "#test-input").
    [DESCRIPTOR_PATH]        Path to host descriptor JSON file or UDS socket.
                             If omitted, auto-discovers the active running Blitz window.

OPTIONS:
    --node, -n <NODE_ID>     Target specific root node/subtree ID
    --selector, -s <SEL>     Target specific root element by CSS selector
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

OUTPUT:
    Always returns typed InspectResponse JSON on stdout.

EXAMPLES:
    blitz-host inspect
    blitz-host inspect 4294967464
    blitz-host inspect "#test-input"
    blitz-host inspect --selector "#test-input"
    blitz-host inspect --pid 37462
"##
    );
}

fn print_click_help() {
    println!(
        r##"blitz-host-mouse-click: Dispatch synthetic click to a live Blitz window element.

USAGE:
    blitz-host mouse click <TARGET> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <TARGET>                 Target element specified by numeric node ID or CSS selector
                             (e.g. 4294967402, "#test-interaction-button")
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching the click
    to ensure reactive state changes and layout recalculations have completed.
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host mouse click 4294967402
    blitz-host mouse click "#test-interaction-button"
    blitz-host mouse click --selector "#test-interaction-button" --pid 37462
"##
    );
}

fn print_focus_help() {
    println!(
        r##"blitz-host-focus: Focus a target element in a live Blitz window.

USAGE:
    blitz-host focus <TARGET> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <TARGET>                 Target element specified by numeric node ID or CSS selector
                             (e.g. 4294967405, "#test-input")
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching focus
    to ensure focus styling and event propagation have completed.
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host focus 4294967405
    blitz-host focus "#test-input"
    blitz-host focus --selector "#test-input" --pid 37462
"##
    );
}

fn print_set_value_help() {
    println!(
        r##"blitz-host-set-value: Set text value on an input element in a live Blitz window.

USAGE:
    blitz-host set-value <TARGET> <VALUE> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <TARGET>                 Target element specified by numeric node ID or CSS selector
                             (e.g. 4294967405, "#test-input")
    <VALUE>                  Text string to inject into the input element
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after setting the value
    to ensure reactive signal updates and layout recalculations have completed.
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host set-value 4294967405 "Hello Blitz"
    blitz-host set-value "#test-input" "Hello Blitz"
    blitz-host set-value --selector "#test-input" "Hello Blitz" --pid 37462
"##
    );
}

fn print_key_help() {
    println!(
        r##"blitz-host-key: Dispatch a synthetic key event to a live Blitz window.

USAGE:
    blitz-host key <KEY_SPEC> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <KEY_SPEC>               Key name or compound specifier:
                             - Navigation & Actions: tab, enter, return, space, escape, esc, backspace, delete, del
                             - Arrow Keys: arrowleft, left, arrowright, right, arrowup, up, arrowdown, down
                             - Shortcuts: cmd+a, command+shift+z, ctrl+c, alt+arrowup
                             - Modifiers supported in specifier: cmd, command, meta, super, shift, ctrl, control, alt, opt, option
                             (Tokens and named keys are case-insensitive)
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Automatically settles 2 VSync frames after dispatching the key
    to ensure reactive updates and layout recalculations have completed.
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    # 1. Focus traversal & navigation
    blitz-host key tab
    blitz-host key shift+tab
    blitz-host key escape
    blitz-host key enter

    # 2. Text editing & Shortcuts
    blitz-host key cmd+a
    blitz-host key command+shift+z
    blitz-host key a --selector "#test-input"
    blitz-host key backspace --node 4294967405
"##
    );
}

fn print_mouse_namespace_help() {
    println!(
        r##"blitz-host-mouse: Pointer and mouse interaction controls for live Blitz windows.

USAGE:
    blitz-host mouse <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    click <TARGET> [OPTIONS]       Dispatch synthetic click to element (auto-settles)
    move [TARGET] [OPTIONS]        Move cursor to element or explicit coordinates (triggers hover)
    down [TARGET] [OPTIONS]        Press mouse button down on element or coordinates
    up [TARGET] [OPTIONS]          Release mouse button on element or coordinates
    wheel [TARGET] [OPTIONS]       Dispatch mouse wheel / scroll delta (requires --dy)
    drag <FROM> <TO> [OPTIONS]     Execute drag sequence between elements (IDs or selectors)

OPTIONS:
    -h, --help                     Print help information

EXAMPLES:
    # 1. Click an element via selector or ID
    blitz-host mouse click "#test-interaction-button"
    blitz-host mouse click 4294967402

    # 2. Hover over an element
    blitz-host mouse move "#mouse-test-card"
    blitz-host mouse move 4294967464

    # 3. Move to explicit window coordinates
    blitz-host mouse move --x 150 --y 200

    # 4. Press and release mouse button
    blitz-host mouse down "#mouse-test-card"
    blitz-host mouse up "#mouse-test-card"

    # 5. Scroll vertically by 50px
    blitz-host mouse wheel "#test-scroll-container" --dy 50
    blitz-host mouse wheel 4294967473 --dy 50

    # 6. Drag and drop from one element to another
    blitz-host mouse drag "#mouse-test-card" "#test-input"
"##
    );
}

fn print_move_help() {
    println!(
        r##"blitz-host-move: Move mouse / hover pointer over a live Blitz window element or coordinates.

USAGE:
    blitz-host mouse move [TARGET] [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]                 Target element specified by numeric node ID or CSS selector
                             (e.g. 4294967402, "#mouse-test-card"). If provided, cursor moves to element center.
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --x <X>              Explicit X coordinate in CSS pixels
        --y <Y>              Explicit Y coordinate in CSS pixels
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host mouse move "#mouse-test-card"
    blitz-host mouse move 4294967402
    blitz-host mouse move --x 150 --y 200
    blitz-host mouse move "#mouse-test-card" --pid 37462
"##
    );
}

fn print_down_help() {
    println!(
        r##"blitz-host-down: Press mouse button down on a live Blitz window element or coordinates.

USAGE:
    blitz-host mouse down [TARGET] [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]                 Target element specified by numeric node ID or CSS selector
                             (e.g. 4294967402, "#mouse-test-card")
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --button <BUTTON>    Mouse button: left, right, middle (default: left)
        --x <X>              Explicit X coordinate in CSS pixels
        --y <Y>              Explicit Y coordinate in CSS pixels
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host mouse down "#mouse-test-card"
    blitz-host mouse down 4294967402 --button right
"##
    );
}

fn print_up_help() {
    println!(
        r##"blitz-host-up: Release mouse button on a live Blitz window element or coordinates.

USAGE:
    blitz-host mouse up [TARGET] [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]                 Target element specified by numeric node ID or CSS selector
                             (e.g. 4294967402, "#mouse-test-card")
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --button <BUTTON>    Mouse button: left, right, middle (default: left)
        --x <X>              Explicit X coordinate in CSS pixels
        --y <Y>              Explicit Y coordinate in CSS pixels
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host mouse up "#mouse-test-card"
    blitz-host mouse up 4294967402
"##
    );
}

fn print_wheel_help() {
    println!(
        r##"blitz-host-wheel: Dispatch mouse wheel / scroll delta on a live Blitz window element.

USAGE:
    blitz-host mouse wheel [TARGET] [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]                 Target element specified by numeric node ID or CSS selector
                             (optional, scrolls target element or hover target)
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
    --node, -n <NODE_ID>     Target specific node ID
    --selector, -s <SEL>     Target specific element by CSS selector
        --dy <DY>            Vertical scroll delta in pixels (e.g. 50, -50)
        --dx <DX>            Horizontal scroll delta in pixels (default: 0)
        --x <X>              Explicit X coordinate in CSS pixels
        --y <Y>              Explicit Y coordinate in CSS pixels
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host mouse wheel "#test-scroll-container" --dy 50
    blitz-host mouse wheel 4294967402 --dy 50
    blitz-host mouse wheel "#test-scroll-container" --dy -30 --pid 37462
"##
    );
}

fn print_drag_help() {
    println!(
        r##"blitz-host-drag: Execute drag sequence between elements (down -> move -> up).

USAGE:
    blitz-host mouse drag <FROM_TARGET> <TO_TARGET> [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    <FROM_TARGET>            Source element specified by numeric node ID or CSS selector
    <TO_TARGET>              Destination element specified by numeric node ID or CSS selector
    [DESCRIPTOR_PATH]        Path to host descriptor JSON (auto-discovered if omitted)

OPTIONS:
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (optional, defaults to primary window)
    -h, --help               Print help information

NOTE:
    Returns typed ActionResponse JSON on stdout.

EXAMPLES:
    blitz-host mouse drag "#mouse-test-card" "#test-input"
    blitz-host mouse drag 4294967402 4294967410
"##
    );
}

fn parse_coords_arg(args: &[String]) -> (Option<f32>, Option<f32>) {
    let mut x = None;
    let mut y = None;
    for i in 0..args.len() {
        if args[i] == "--x" && i + 1 < args.len() {
            x = args[i + 1].parse().ok();
        } else if let Some(rest) = args[i].strip_prefix("--x=") {
            x = rest.parse().ok();
        }
        if args[i] == "--y" && i + 1 < args.len() {
            y = args[i + 1].parse().ok();
        } else if let Some(rest) = args[i].strip_prefix("--y=") {
            y = rest.parse().ok();
        }
    }
    (x, y)
}

fn parse_button_arg(args: &[String]) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == "--button" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        if let Some(rest) = args[i].strip_prefix("--button=") {
            return Some(rest.to_string());
        }
    }
    None
}

fn parse_delta_args(args: &[String]) -> (f64, f64) {
    let mut dx = 0.0f64;
    let mut dy = 0.0f64;
    for i in 0..args.len() {
        if (args[i] == "--dx" || args[i] == "--delta-x") && i + 1 < args.len() {
            dx = args[i + 1].parse().unwrap_or(0.0);
        } else if let Some(rest) = args[i].strip_prefix("--dx=").or_else(|| args[i].strip_prefix("--delta-x=")) {
            dx = rest.parse().unwrap_or(0.0);
        }
        if (args[i] == "--dy" || args[i] == "--delta-y") && i + 1 < args.len() {
            dy = args[i + 1].parse().unwrap_or(0.0);
        } else if let Some(rest) = args[i].strip_prefix("--dy=").or_else(|| args[i].strip_prefix("--delta-y=")) {
            dy = rest.parse().unwrap_or(0.0);
        }
    }
    (dx, dy)
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

fn parse_target_flag(args: &[String]) -> Option<ElementTarget> {
    for i in 0..args.len() {
        if (args[i] == "--selector" || args[i] == "-s" || args[i] == "--node" || args[i] == "-n") && i + 1 < args.len() {
            return Some(ElementTarget::from(args[i + 1].as_str()));
        }
        if let Some(rest) = args[i]
            .strip_prefix("--selector=")
            .or_else(|| args[i].strip_prefix("-s="))
            .or_else(|| args[i].strip_prefix("--node="))
            .or_else(|| args[i].strip_prefix("-n="))
        {
            return Some(ElementTarget::from(rest));
        }
    }
    None
}

#[allow(dead_code)]
fn parse_node_arg(args: &[String]) -> Option<u64> {
    parse_target_flag(args).and_then(|t| match t {
        ElementTarget::Id(id) => Some(id),
        _ => None,
    })
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

/// Connect to Blitz host matching the selector, exiting cleanly with code 1 if connection or ambiguity fails.
fn connect_cli_client(selector: &TargetSelector) -> DebugClient {
    match DebugClient::connect_target(selector) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error connecting to Blitz host: {e}");
            eprintln!("Use 'blitz-host list' to inspect available hosts.");
            std::process::exit(1);
        }
    }
}


/// Helper to parse compound key expressions like `cmd+a`, `command+shift+z`, `shift+tab`.
fn parse_compound_key(raw: &str) -> (String, Option<KeyModifiers>) {
    let mut shift = false;
    let mut ctrl = false;
    let mut alt = false;
    let mut meta = false;
    let mut key_part = raw.trim();

    while let Some(idx) = key_part.find('+') {
        let prefix = key_part[..idx].trim();
        if prefix.eq_ignore_ascii_case("shift") {
            shift = true;
        } else if prefix.eq_ignore_ascii_case("ctrl") || prefix.eq_ignore_ascii_case("control") {
            ctrl = true;
        } else if prefix.eq_ignore_ascii_case("cmd")
            || prefix.eq_ignore_ascii_case("command")
            || prefix.eq_ignore_ascii_case("meta")
            || prefix.eq_ignore_ascii_case("super")
        {
            meta = true;
        } else if prefix.eq_ignore_ascii_case("alt")
            || prefix.eq_ignore_ascii_case("opt")
            || prefix.eq_ignore_ascii_case("option")
        {
            alt = true;
        }
        key_part = key_part[idx + 1..].trim();
    }

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

    (key_part.to_string(), modifiers)
}

fn handle_move_command(subargs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if subargs.iter().any(|a| a == "-h" || a == "--help") {
        print_move_help();
        return Ok(());
    }

    let mut positional = Vec::new();
    let mut skip_next = false;
    for arg in subargs {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--pid"
            || arg == "--window"
            || arg == "--window-id"
            || arg == "--x"
            || arg == "--y"
            || arg == "--node"
            || arg == "-n"
            || arg == "--selector"
            || arg == "-s"
        {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--pid=")
            || arg.starts_with("--window=")
            || arg.starts_with("--window-id=")
            || arg.starts_with("--x=")
            || arg.starts_with("--y=")
            || arg.starts_with("--node=")
            || arg.starts_with("-n=")
            || arg.starts_with("--selector=")
            || arg.starts_with("-s=")
            || arg.starts_with('-')
        {
            continue;
        }
        if arg.ends_with(".json") || arg.ends_with(".sock") {
            continue;
        }
        positional.push(arg.as_str());
    }

    let target = parse_target_flag(subargs)
        .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

    let (cx, cy) = parse_coords_arg(subargs);
    let coords = match (cx, cy) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };

    if target.is_none() && coords.is_none() {
        eprintln!("Error: 'move' requires either a target <TARGET> (node ID or CSS selector) or coordinates (--x and --y).");
        eprintln!("Run 'blitz-host mouse move --help' for usage.");
        std::process::exit(1);
    }

    let window_id = parse_window_arg(subargs);
    let selector = determine_selector(subargs);

    let mut client = connect_cli_client(&selector);

    eprintln!(
        "Dispatching pointer move (target: {:?}, coords: {:?}) (PID: {})...",
        target,
        coords,
        client.descriptor().pid
    );
    let act_res = client.mouse_move_target(window_id, target, coords, None)?;
    let _ = client.settle_window(window_id, 2)?;
    println!("{}", serde_json::to_string_pretty(&act_res)?);
    if !act_res.success {
        if let Some(msg) = &act_res.message {
            eprintln!("Error: {msg}");
        }
        std::process::exit(1);
    }
    Ok(())
}

fn handle_down_command(subargs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if subargs.iter().any(|a| a == "-h" || a == "--help") {
        print_down_help();
        return Ok(());
    }

    let mut positional = Vec::new();
    let mut skip_next = false;
    for arg in subargs {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--pid"
            || arg == "--window"
            || arg == "--window-id"
            || arg == "--x"
            || arg == "--y"
            || arg == "--button"
            || arg == "--node"
            || arg == "-n"
            || arg == "--selector"
            || arg == "-s"
        {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--pid=")
            || arg.starts_with("--window=")
            || arg.starts_with("--window-id=")
            || arg.starts_with("--x=")
            || arg.starts_with("--y=")
            || arg.starts_with("--button=")
            || arg.starts_with("--node=")
            || arg.starts_with("-n=")
            || arg.starts_with("--selector=")
            || arg.starts_with("-s=")
            || arg.starts_with('-')
        {
            continue;
        }
        if arg.ends_with(".json") || arg.ends_with(".sock") {
            continue;
        }
        positional.push(arg.as_str());
    }

    let target = parse_target_flag(subargs)
        .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

    let (cx, cy) = parse_coords_arg(subargs);
    let coords = match (cx, cy) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    let button = parse_button_arg(subargs);

    let window_id = parse_window_arg(subargs);
    let selector = determine_selector(subargs);

    let mut client = connect_cli_client(&selector);

    eprintln!(
        "Dispatching pointer down (target: {:?}, coords: {:?}, button: {:?}) (PID: {})...",
        target,
        coords,
        button,
        client.descriptor().pid
    );
    let act_res = client.mouse_down_target(window_id, target, coords, button.as_deref(), None)?;
    let _ = client.settle_window(window_id, 2)?;
    println!("{}", serde_json::to_string_pretty(&act_res)?);
    if !act_res.success {
        if let Some(msg) = &act_res.message {
            eprintln!("Error: {msg}");
        }
        std::process::exit(1);
    }
    Ok(())
}

fn handle_up_command(subargs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if subargs.iter().any(|a| a == "-h" || a == "--help") {
        print_up_help();
        return Ok(());
    }

    let mut positional = Vec::new();
    let mut skip_next = false;
    for arg in subargs {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--pid"
            || arg == "--window"
            || arg == "--window-id"
            || arg == "--x"
            || arg == "--y"
            || arg == "--button"
            || arg == "--node"
            || arg == "-n"
            || arg == "--selector"
            || arg == "-s"
        {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--pid=")
            || arg.starts_with("--window=")
            || arg.starts_with("--window-id=")
            || arg.starts_with("--x=")
            || arg.starts_with("--y=")
            || arg.starts_with("--button=")
            || arg.starts_with("--node=")
            || arg.starts_with("-n=")
            || arg.starts_with("--selector=")
            || arg.starts_with("-s=")
            || arg.starts_with('-')
        {
            continue;
        }
        if arg.ends_with(".json") || arg.ends_with(".sock") {
            continue;
        }
        positional.push(arg.as_str());
    }

    let target = parse_target_flag(subargs)
        .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

    let (cx, cy) = parse_coords_arg(subargs);
    let coords = match (cx, cy) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    let button = parse_button_arg(subargs);

    let window_id = parse_window_arg(subargs);
    let selector = determine_selector(subargs);

    let mut client = connect_cli_client(&selector);

    eprintln!(
        "Dispatching pointer up (target: {:?}, coords: {:?}, button: {:?}) (PID: {})...",
        target,
        coords,
        button,
        client.descriptor().pid
    );
    let act_res = client.mouse_up_target(window_id, target, coords, button.as_deref(), None)?;
    let _ = client.settle_window(window_id, 2)?;
    println!("{}", serde_json::to_string_pretty(&act_res)?);
    if !act_res.success {
        if let Some(msg) = &act_res.message {
            eprintln!("Error: {msg}");
        }
        std::process::exit(1);
    }
    Ok(())
}

fn handle_wheel_command(subargs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if subargs.iter().any(|a| a == "-h" || a == "--help") {
        print_wheel_help();
        return Ok(());
    }

    let mut positional = Vec::new();
    let mut skip_next = false;
    for arg in subargs {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--pid"
            || arg == "--window"
            || arg == "--window-id"
            || arg == "--x"
            || arg == "--y"
            || arg == "--dx"
            || arg == "--dy"
            || arg == "--delta-x"
            || arg == "--delta-y"
            || arg == "--node"
            || arg == "-n"
            || arg == "--selector"
            || arg == "-s"
        {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--pid=")
            || arg.starts_with("--window=")
            || arg.starts_with("--window-id=")
            || arg.starts_with("--x=")
            || arg.starts_with("--y=")
            || arg.starts_with("--dx=")
            || arg.starts_with("--dy=")
            || arg.starts_with("--delta-x=")
            || arg.starts_with("--delta-y=")
            || arg.starts_with("--node=")
            || arg.starts_with("-n=")
            || arg.starts_with("--selector=")
            || arg.starts_with("-s=")
            || arg.starts_with('-')
        {
            continue;
        }
        if arg.ends_with(".json") || arg.ends_with(".sock") {
            continue;
        }
        positional.push(arg.as_str());
    }

    let target = parse_target_flag(subargs)
        .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

    let (cx, cy) = parse_coords_arg(subargs);
    let coords = match (cx, cy) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    let (dx, dy) = parse_delta_args(subargs);

    if dy == 0.0 && dx == 0.0 {
        eprintln!("Error: 'wheel' requires a non-zero scroll delta via '--dy <DELTA_Y>' or '--dx <DELTA_X>'.");
        eprintln!("Run 'blitz-host mouse wheel --help' for usage.");
        std::process::exit(1);
    }

    let window_id = parse_window_arg(subargs);
    let selector = determine_selector(subargs);

    let mut client = connect_cli_client(&selector);

    eprintln!(
        "Dispatching mouse wheel (target: {:?}, dx: {}, dy: {}, coords: {:?}) (PID: {})...",
        target,
        dx,
        dy,
        coords,
        client.descriptor().pid
    );
    let act_res = client.wheel_target(window_id, target, coords, dx, dy, None)?;
    let _ = client.settle_window(window_id, 2)?;
    println!("{}", serde_json::to_string_pretty(&act_res)?);
    if !act_res.success {
        if let Some(msg) = &act_res.message {
            eprintln!("Error: {msg}");
        }
        std::process::exit(1);
    }
    Ok(())
}

fn handle_drag_command(subargs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if subargs.iter().any(|a| a == "-h" || a == "--help") {
        print_drag_help();
        return Ok(());
    }

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

    if positional.len() < 2 {
        eprintln!("Error: 'drag' requires <FROM_TARGET> and <TO_TARGET> positional arguments (node IDs or CSS selectors).");
        eprintln!("Run 'blitz-host mouse drag --help' for usage.");
        std::process::exit(1);
    }

    let from_target = ElementTarget::from(positional[0]);
    let to_target = ElementTarget::from(positional[1]);

    let window_id = parse_window_arg(subargs);
    let selector = determine_selector(subargs);

    let mut client = connect_cli_client(&selector);

    eprintln!(
        "Dispatching drag sequence from target '{}' to target '{}' (PID: {})...",
        from_target,
        to_target,
        client.descriptor().pid
    );
    let act_res = client.drag_target(from_target, to_target)?;
    let _ = client.settle_window(window_id, 2)?;
    println!("{}", serde_json::to_string_pretty(&act_res)?);
    if !act_res.success {
        if let Some(msg) = &act_res.message {
            eprintln!("Error: {msg}");
        }
        std::process::exit(1);
    }
    Ok(())
}

fn handle_click_command(subargs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if subargs.iter().any(|a| a == "-h" || a == "--help") {
        print_click_help();
        return Ok(());
    }

    let mut positional = Vec::new();
    let mut skip_next = false;
    for arg in subargs {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--pid"
            || arg == "--window"
            || arg == "--window-id"
            || arg == "--node"
            || arg == "-n"
            || arg == "--selector"
            || arg == "-s"
        {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--pid=")
            || arg.starts_with("--window=")
            || arg.starts_with("--window-id=")
            || arg.starts_with("--node=")
            || arg.starts_with("-n=")
            || arg.starts_with("--selector=")
            || arg.starts_with("-s=")
            || arg.starts_with('-')
        {
            continue;
        }
        if arg.ends_with(".json") || arg.ends_with(".sock") {
            continue;
        }
        positional.push(arg.as_str());
    }

    let target = parse_target_flag(subargs)
        .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

    let target = match target {
        Some(t) => t,
        None => {
            eprintln!("Error: 'click' requires a target <TARGET> argument (node ID or CSS selector).");
            eprintln!("Run 'blitz-host mouse click --help' for usage.");
            std::process::exit(1);
        }
    };

    let window_id = parse_window_arg(subargs);
    let selector = determine_selector(subargs);

    let mut client = connect_cli_client(&selector);

    eprintln!(
        "Dispatching click action to target '{}' (PID: {})...",
        target,
        client.descriptor().pid
    );
    let act_res = client.click_target_window(window_id, target)?;
    let _settle_res = client.settle_window(window_id, 2)?;
    println!("{}", serde_json::to_string_pretty(&act_res)?);
    if !act_res.success {
        if let Some(msg) = &act_res.message {
            eprintln!("Error: {msg}");
        }
        std::process::exit(1);
    }
    Ok(())
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

            let hosts = blitz_host::transport::list_hosts()?;
            println!("{}", serde_json::to_string_pretty(&hosts)?);
            Ok(())
        }
        "inspect" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_inspect_help();
                return Ok(());
            }

            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            // Extract optional target from `--node <ID>` / `-n <ID>` / `--selector <SEL>` / `-s <SEL>` or positional argument
            let mut positional = Vec::new();
            let mut skip_next = false;
            for arg in subargs {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "--node"
                    || arg == "-n"
                    || arg == "--selector"
                    || arg == "-s"
                    || arg == "--pid"
                    || arg == "--window"
                    || arg == "--window-id"
                {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with('-') || arg.ends_with(".sock") || arg.ends_with(".json") {
                    continue;
                }
                positional.push(arg.as_str());
            }

            let target = parse_target_flag(subargs)
                .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

            let mut client = connect_cli_client(&selector);

            let mut req = InspectRequest {
                window_id,
                ..Default::default()
            };
            if let Some(t) = target {
                match t {
                    ElementTarget::Id(id) => req.root_node_id = Some(id),
                    ElementTarget::Selector(sel) => req.selector = Some(sel),
                }
            }
            let response = client.inspect(req)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            if response.node_count == 0 && response.message.is_some() {
                if let Some(msg) = &response.message {
                    eprintln!("Error: {msg}");
                }
                std::process::exit(1);
            }
            Ok(())
        }
        "capture" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_capture_help();
                return Ok(());
            }

            let output_path = parse_output_arg(subargs);
            let target_file = match output_path {
                Some(p) => p,
                None => {
                    eprintln!("Error: Output path is required. Use '-o <PATH>' or '--output <PATH>' to specify where to save the screenshot.");
                    eprintln!("Run 'blitz-host capture --help' for usage.");
                    std::process::exit(1);
                }
            };

            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            // Extract optional target from `--node` / `-n` / `--selector` / `-s` or positional argument
            let mut positional = Vec::new();
            let mut skip_next = false;
            for arg in subargs {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "-o"
                    || arg == "--output"
                    || arg == "--node"
                    || arg == "-n"
                    || arg == "--selector"
                    || arg == "-s"
                    || arg == "--pid"
                    || arg == "--window"
                    || arg == "--window-id"
                {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with('-') || arg.ends_with(".sock") || arg.ends_with(".json") {
                    continue;
                }
                positional.push(arg.as_str());
            }

            let target = parse_target_flag(subargs)
                .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

            let mut client = connect_cli_client(&selector);

            let resp = match target {
                Some(t) => client.capture_target_window(window_id, t, &target_file),
                None => client.capture_window(window_id, &target_file),
            };

            let resp = match resp {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("Error capturing visual screenshot: {err}");
                    std::process::exit(1);
                }
            };

            if !resp.success {
                eprintln!(
                    "Capture failed: {}",
                    resp.message.as_deref().unwrap_or("unknown error")
                );
                std::process::exit(1);
            }

            println!("{}", serde_json::to_string_pretty(&resp)?);
            Ok(())
        }
        "focus" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_focus_help();
                return Ok(());
            }

            let mut positional = Vec::new();
            let mut skip_next = false;
            for arg in subargs {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "--pid"
                    || arg == "--window"
                    || arg == "--window-id"
                    || arg == "--node"
                    || arg == "-n"
                    || arg == "--selector"
                    || arg == "-s"
                {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with("--pid=")
                    || arg.starts_with("--window=")
                    || arg.starts_with("--window-id=")
                    || arg.starts_with("--node=")
                    || arg.starts_with("-n=")
                    || arg.starts_with("--selector=")
                    || arg.starts_with("-s=")
                    || arg.starts_with('-')
                {
                    continue;
                }
                if arg.ends_with(".json") || arg.ends_with(".sock") {
                    continue;
                }
                positional.push(arg.as_str());
            }

            let target = parse_target_flag(subargs)
                .or_else(|| positional.first().map(|s| ElementTarget::from(*s)));

            let target = match target {
                Some(t) => t,
                None => {
                    eprintln!("Error: 'focus' requires a target <TARGET> argument (node ID or CSS selector).");
                    eprintln!("Run 'blitz-host focus --help' for usage.");
                    std::process::exit(1);
                }
            };

            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            let mut client = connect_cli_client(&selector);

            eprintln!(
                "Dispatching focus action to target '{}' (PID: {})...",
                target,
                client.descriptor().pid
            );
            let act_res = client.focus_target_window(window_id, target)?;
            let _settle_res = client.settle_window(window_id, 2)?;
            println!("{}", serde_json::to_string_pretty(&act_res)?);
            if !act_res.success {
                if let Some(msg) = &act_res.message {
                    eprintln!("Error: {msg}");
                }
                std::process::exit(1);
            }
            Ok(())
        }
        "set-value" => {
            let subargs = &args[2..];
            if subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_set_value_help();
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
                if arg == "--pid"
                    || arg == "--window"
                    || arg == "--window-id"
                    || arg == "--node"
                    || arg == "-n"
                    || arg == "--selector"
                    || arg == "-s"
                {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with("--pid=")
                    || arg.starts_with("--window=")
                    || arg.starts_with("--window-id=")
                    || arg.starts_with("--node=")
                    || arg.starts_with("-n=")
                    || arg.starts_with("--selector=")
                    || arg.starts_with("-s=")
                    || arg.starts_with('-')
                {
                    continue;
                }
                if arg.ends_with(".json") || arg.ends_with(".sock") {
                    continue;
                }
                positional.push(arg.as_str());
            }

            let flag_target = parse_target_flag(subargs);
            let (target, value) = match flag_target {
                Some(t) => {
                    let val = positional.first().copied().unwrap_or("").to_string();
                    (t, val)
                }
                None => {
                    if positional.is_empty() {
                        eprintln!("Error: 'set-value' requires target <TARGET> and <VALUE> arguments (e.g. '#test-input' 'hello').");
                        eprintln!("Run 'blitz-host set-value --help' for usage.");
                        std::process::exit(1);
                    }
                    let t = ElementTarget::from(positional[0]);
                    let val = if positional.len() > 1 {
                        positional[1].to_string()
                    } else {
                        String::new()
                    };
                    (t, val)
                }
            };

            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            let mut client = connect_cli_client(&selector);

            eprintln!(
                "Dispatching set-value action (value: {:?}) to target '{}' (PID: {})...",
                value,
                target,
                client.descriptor().pid
            );
            let act_res = client.set_value_target_window(window_id, target, value)?;
            let _settle_res = client.settle_window(window_id, 2)?;
            println!("{}", serde_json::to_string_pretty(&act_res)?);
            if !act_res.success {
                if let Some(msg) = &act_res.message {
                    eprintln!("Error: {msg}");
                }
                std::process::exit(1);
            }
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
                if arg == "--pid"
                    || arg == "--window"
                    || arg == "--window-id"
                    || arg == "--node"
                    || arg == "-n"
                    || arg == "--selector"
                    || arg == "-s"
                {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with("--pid=")
                    || arg.starts_with("--window=")
                    || arg.starts_with("--window-id=")
                    || arg.starts_with("--node=")
                    || arg.starts_with("-n=")
                    || arg.starts_with("--selector=")
                    || arg.starts_with("-s=")
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
                eprintln!("Error: 'key' requires a <KEY_SPEC> argument (e.g. cmd+a, shift+tab, enter, escape).");
                eprintln!("Run 'blitz-host key --help' for usage.");
                std::process::exit(1);
            }

            let raw_key_str = positional[0];
            let (key_str, modifiers) = parse_compound_key(raw_key_str);
            let target = parse_target_flag(subargs);
            let window_id = parse_window_arg(subargs);
            let selector = determine_selector(subargs);

            let mut client = connect_cli_client(&selector);

            eprintln!(
                "Dispatching key action '{key_str}' (target: {:?}, modifiers: {:?}) (PID: {})...",
                target,
                modifiers,
                client.descriptor().pid
            );
            let act_res = client.key_target(window_id, target, &key_str, modifiers)?;
            let _settle_res = client.settle_window(window_id, 2)?;
            println!("{}", serde_json::to_string_pretty(&act_res)?);
            if !act_res.success {
                if let Some(msg) = &act_res.message {
                    eprintln!("Error: {msg}");
                }
                std::process::exit(1);
            }
            Ok(())
        }
        "mouse" => {
            let subargs = &args[2..];
            if subargs.is_empty() || subargs.iter().any(|a| a == "-h" || a == "--help") {
                print_mouse_namespace_help();
                return Ok(());
            }

            let mouse_subcmd = subargs[0].as_str();
            let mouse_subargs = &subargs[1..];
            match mouse_subcmd {
                "click" => handle_click_command(mouse_subargs)?,
                "move" => handle_move_command(mouse_subargs)?,
                "down" => handle_down_command(mouse_subargs)?,
                "up" => handle_up_command(mouse_subargs)?,
                "wheel" => handle_wheel_command(mouse_subargs)?,
                "drag" => handle_drag_command(mouse_subargs)?,
                "-h" | "--help" => {
                    print_mouse_namespace_help();
                    return Ok(());
                }
                other => {
                    eprintln!("Unknown mouse subcommand: '{other}'. Use 'blitz-host mouse --help' for available subcommands.");
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        "click" | "move" | "down" | "up" | "wheel" | "drag" => {
            eprintln!("Error: '{subcmd}' is a pointer command and belongs under the 'mouse' namespace. Run 'blitz-host mouse {subcmd} ...' instead.");
            std::process::exit(1);
        }
        other => {
            eprintln!("Unknown subcommand: '{other}'. Use 'blitz-host --help' for available subcommands.");
            std::process::exit(1);
        }
    }
}
