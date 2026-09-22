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
- **Zero-Overhead Decoupling**: 3-crate split ensures protocol consumers (clients/runners) never need to compile Blitz, Winit, or GPU renderers.

---

## Workspace Crates

| Crate | Purpose | Dependencies |
|---|---|---|
| [`blitz-host-protocol`](crates/blitz-host-protocol) | Pure typed request/response schema (`Inspect`, `Act`, `Settle`, `SemanticNode`) | `serde`, `serde_json` only |
| [`blitz-host-transport`](crates/blitz-host-transport) | Local UDS server, client, discovery, and canonical CLI control tool (`blitz-host`) | `blitz-host-protocol`, `serde`, `libc` |
| [`blitz-host-bridge`](crates/blitz-host-bridge) | UI-thread adapter bridging `blitz_dom::BaseDocument` and frame-settle queues | `blitz-host-protocol`, `blitz-host-transport`, `blitz-dom` |

---

## Quick Start

### 1. Attaching and Inspecting a Running Host

```rust
use blitz_host_transport::DebugClient;
use blitz_host_protocol::InspectRequest;

// Discover and attach to the most recent running Blitz host
let mut client = DebugClient::connect_discovered(None)?;

// Inspect live semantic tree
let snapshot = client.inspect(InspectRequest::default())?;
println!("Document ID: {}, Total Nodes: {}", snapshot.document_id, snapshot.node_count);

for node in &snapshot.nodes {
    if let Some(bounds) = node.bounds {
        println!("Node #{} <{}> rect: {:?}", node.id, node.tag, bounds);
    }
}
```

### 2. Dispatching Actions and Deterministic Frame Settling

```rust
use std::time::Duration;

// Find target button node
let button = snapshot.nodes.iter().find(|n| n.tag == "button").unwrap();

// Send typed click action through native event plumbing
client.click(button.id)?;

// Synchronize: wait for 2 VSync frames to allow state & DOM mutation
client.settle(2)?;

// Definitive proof: wait until the label reflects the updated state
let updated_snapshot = client.settle_until(Duration::from_secs(3), |snap| {
    snap.nodes.iter().any(|n| n.text.as_deref() == Some("Clicked 1 times"))
})?;
```

---

## Canonical CLI Control Tool: `blitz-host`

`blitz-host-transport` provides the standalone `blitz-host` binary:

```bash
# 1. Inspect live Blitz window (default)
cargo run -p blitz-host-transport --bin blitz-host

# 2. Output live DOM hierarchy as JSON
cargo run -p blitz-host-transport --bin blitz-host -- --json

# 3. Dispatch click action to a specific node on the live window
cargo run -p blitz-host-transport --bin blitz-host -- click 4294967402

# 4. View help
cargo run -p blitz-host-transport --bin blitz-host -- --help
```

Output:
```text
=================================================================
[blitz-host] Blitz Host Live Inspector & Controller
=================================================================
Connected to host:
  • Renderer        : oxidase-native-runner v0.1.0
  • PID             : 9544
  • Socket Path     : /tmp/blitz-host/9544-1789997960602929000.sock
-----------------------------------------------------------------
Received typed InspectResponse:
  • Document ID: 1
  • Root ID    : 4294967297
  • Node Count : 66
-----------------------------------------------------------------
Hierarchy:
#4294967297 <#document>
  #4294967298 <html> rect(0.0, 0.0, 800.0, 600.0)
    #4294967300 <body> rect(8.0, 8.0, 784.0, 584.0)
      #4294967402 <button> id="test-interaction-button" rect(493.0, 0.0, 125.0, 27.0)
```

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
