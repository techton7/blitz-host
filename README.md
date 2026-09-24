# blitz-host

[![Release-plz](https://github.com/techton7/blitz-host/actions/workflows/release-plz.yml/badge.svg)](https://github.com/techton7/blitz-host/actions/workflows/release-plz.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**Out-of-process control plane, live DOM inspection harness, and deterministic frame driver for Blitz & Dioxus Native desktop applications.**

`blitz-host` provides an external control plane (comparable to Chrome DevTools Protocol or Playwright, but purpose-built for the native Blitz/Winit engine). It enables external test runners, CLI tools, and autonomous AI coding agents to attach to a running desktop application, inspect computed layout and semantic DOM trees, dispatch typed UI actions via native event plumbing, and deterministically synchronize with VSync frame execution.

---

## Key Capabilities

- **Local-First & Owner-Only Discovery**: Attaches over Unix Domain Sockets (UDS) with strict file permissions (`0o600` descriptor files and sockets in `$TMPDIR/blitz-host/`), dead PID reaping, and zero open network ports.
- **UI-Thread Safe DOM Inspection**: Safely traverses `blitz_dom::BaseDocument` on the main UI thread, extracting semantic tags, DOM IDs, WAI-ARIA roles, text nodes, and exact computed Stylo layout bounds (`[x, y, width, height]`).
- **Native Event Action Dispatching**: Executes typed synthetic actions (`Click { node_id }`) through `dioxus-native-dom`'s own event pipeline (`synthetic_click_event`) instead of synthetic coordinate hacks.
- **Deterministic VSync Frame Settling**: Provides `settle(frames)` and `settle_until(timeout, predicate)` to eliminate flaky timers and guarantee that layout passes and reactive state updates have fully settled.
- **Unified Facade & Decoupled Core**: `blitz-host` provides a clean top-level facade and CLI, while keeping protocol, transport, and bridge decoupled internally.

---

## Workspace Crates

| Crate | Purpose | Dependencies |
|---|---|---|
| [`blitz-host`](crates/blitz-host) | **Canonical top-level facade, CLI binary (`blitz-host`), and host lifecycle coordinator** | `protocol`, `transport`, `bridge`, `blitz-dom` |
| [`blitz-host-protocol`](crates/blitz-host-protocol) | Pure typed request/response schema (`Inspect`, `Act`, `Settle`, `SemanticNode`) | `serde`, `serde_json` only |
| [`blitz-host-transport`](crates/blitz-host-transport) | Local UDS server, client, and discovery library | `blitz-host-protocol`, `serde`, `libc` |
| [`blitz-host-bridge`](crates/blitz-host-bridge) | UI-thread adapter bridging `blitz_dom::BaseDocument` and frame-settle queues | `blitz-host-protocol`, `blitz-host-transport`, `blitz-dom` |

---

## Canonical CLI Control Tool: `blitz-host`

Install or run the CLI directly from the `blitz-host` package:

```bash
# Install globally
cargo install blitz-host

# Or run from workspace
cargo run -p blitz-host --bin blitz-host -- [SUBCOMMAND]
```

### 1. View Usage & Subcommands
```bash
blitz-host --help
```

### 2. Inspect Live Window
```bash
# Returns full semantic DOM tree with computed layout bounds as deterministic JSON
blitz-host inspect

# Target a specific host by PID
blitz-host inspect --pid 37462
```

### 3. Visual Screenshot Capture (Mandatory Output File)
```bash
# Full-window capture (returns metadata JSON on stdout; PNG saved to disk)
blitz-host capture -o target/screenshot.png

# Node / Subtree crop capture
blitz-host capture 4294967464 -o target/card.png
blitz-host capture --node 4294967464 -o target/card.png
```

### 4. Interactive UI Actions with Auto-Settle
```bash
# Click an element by node ID
blitz-host click 4294967402

# Focus an input element
blitz-host focus 4294967403

# Set input value
blitz-host set-value 4294967403 "Hello Native"

# Dispatch keyboard shortcut with compound modifiers
blitz-host key cmd+a
blitz-host key shift+tab
blitz-host key enter

# Dispatch pointer events via mouse namespace
blitz-host mouse move 4294967402
blitz-host mouse down 4294967402
blitz-host mouse up 4294967402
blitz-host mouse wheel 4294967402 --dy 50
blitz-host mouse drag 4294967402 4294967410
```

---

## Rust Client API (`DebugClient`)

```rust
use blitz_host::prelude::*;
use std::time::Duration;

// 1. Discover and connect to active Blitz desktop window
let mut client = DebugClient::connect_discovered(None)?;

// 2. Inspect live semantic DOM tree
let snapshot = client.inspect(InspectRequest::default())?;
let button = snapshot.nodes.iter().find(|n| n.tag == "button").unwrap();

// 3. Dispatch click and auto-settle frames
client.click(button.id)?;
client.settle(2)?;

// 4. Verify resulting UI state change deterministically
client.settle_until(Duration::from_secs(3), |snap| {
    snap.nodes.iter().any(|n| n.text.as_deref() == Some("Clicked 1 times"))
})?;
```

---

## Host-Side Integration (`HostControl`)

Host applications can enable `blitz-host` debug control via the `HostControl` facade helper without manual socket or mutex boilerplate:

```rust
use blitz_host::HostControl;

fn main() {
    // 1. Starts UDS debug server (active by default in feature-enabled dev lane)
    HostControl::init_global("my-app", env!("CARGO_PKG_VERSION"));


    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut live_handle = use_signal(|| None::<dioxus_native::NodeHandle>);

    use_frame(move |info| {
        // 2. Service control requests on the UI thread holding the document
        if let Some(handle) = live_handle() {
            HostControl::service_global_frame(&handle.doc(), info.frame_count, |req, doc| {
                HostControl::handle_action_click(req, doc, |d, nid| {
                    dioxus_native::dispatch_synthetic_click(
                        d,
                        blitz_dom::NodeId::from_u64(nid),
                        keyboard_types::Modifiers::empty(),
                    )
                })
            });
        }
    });

    rsx! {
        div {
            onmounted: move |evt| {
                live_handle.set(evt.downcast::<dioxus_native::NodeHandle>().cloned());
            },
            ...
        }
    }
}
```

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
