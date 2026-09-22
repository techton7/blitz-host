# Result: `blitz-host` Facade Architecture & Runner Integration Cleanup

## 1. Executive Summary

We have refactored the previously proven `blitz-host` vertical slice (Inspect, Act, Settle) from a scattered set of 3 low-level internal crates into a unified, product-grade **`blitz-host` facade crate** (`crates/blitz-host`).

The refactoring achieved three critical objectives:
1. **Canonical Facade Crate**: Added `crates/blitz-host` which re-exports `protocol`, `client` (transport), `bridge`, and high-level `host` integration helpers, while housing the canonical `blitz-host` CLI binary.
2. **Materially Cleaner Runner Integration**: Eliminated the ugly, hardcoded multi-crate wiring in `oxidase-native-runner`. Replaced manual server startups, global static mutexes, downcasting plumbing, and match arms with a clean 2-point integration surface (`HostControl::init_global_if_requested` in `main()` and `HostControl::service_global_frame` with `handle_action_click` in `use_frame`).
3. **Preserved 100% Runtime Proof**: Verified live on macOS with real window mount, DOM inspection (66 nodes), synthetic click dispatch, 2-frame VSync settlement, and live button state mutation (`"Click to Test Event"` -> `"Clicked 1 times"` -> `"Clicked 2 times"`).

---

## 2. Architecture & Facade Layout

### A. Facade & Internal Split

```text
util/blitz-host/
├── Cargo.toml
├── README.md
├── ROADMAP.md
├── result.md
└── crates/
    ├── blitz-host/               # 💡 Canonical top-level facade crate & CLI binary
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs            # Facade re-exports (protocol, client, bridge, host)
    │       ├── host.rs           # Host-side lifecycle coordinator & servicing helper
    │       └── bin/
    │           └── blitz-host.rs # Canonical CLI binary (`inspect`, `click`)
    ├── blitz-host-protocol/      # Pure typed JSON request/response schema (zero runtime deps)
    ├── blitz-host-transport/     # Unix Domain Socket transport, discovery, and DebugClient library
    └── blitz-host-bridge/        # UI-thread bridge servicing inspect, act, and settle queues
```

### B. What `blitz-host` Re-exports

From `crates/blitz-host/src/lib.rs`:
- **Underlying crates**:
  - `pub use blitz_host_protocol as protocol;`
  - `pub use blitz_host_transport as client;`
  - `pub use blitz_host_transport as transport;`
  - `pub use blitz_host_bridge as bridge;`
- **Host Integration Helper**:
  - `pub mod host;`
  - `pub use host::HostControl;`
- **Client & Wire Types**:
  - `pub use blitz_host_transport::DebugClient;`
  - `pub use blitz_host_protocol::{ActionRequest, ActionResponse, ControlRequest, ControlResponse, HostDescriptor, InspectRequest, InspectResponse, SettleRequest, SettleResponse};`
- **Prelude**:
  - `blitz_host::prelude::*` for rapid consumer adoption.

### C. Canonical CLI Location

The CLI binary now lives in:
```text
crates/blitz-host/src/bin/blitz-host.rs
```
The previous `[[bin]]` configuration in `crates/blitz-host-transport` was deleted, making `blitz-host-transport` a pure client/transport library. Running `cargo install blitz-host` now produces the canonical `blitz-host` CLI directly.

---

## 3. Host Integration Helper & Runner Cleanup

### A. The `HostControl` Lifecycle Coordinator (`crates/blitz-host/src/host.rs`)

`HostControl` absorbs the low-level boilerplate previously scattered across the host runner:
1. **Detection & Startup**: `HostControl::init_global_if_requested(app_name, app_version)` inspects `--debug-control` and `BLITZ_DEBUG_CONTROL=1`, starts the UDS server, creates the descriptor, and holds the bridge in a thread-safe singleton.
2. **UI-Thread Servicing**: `HostControl::service_global_frame(doc, current_frame, dispatch_action)` services pending inspect, click, and settle requests synchronously against the live `BaseDocument`.
3. **Action Dispatch Helper**: `HostControl::handle_action_click(req, doc, click_fn)` handles matching and response packaging so hosts only provide the raw synthetic click invocation.

### B. Material Reduction in `oxidase-native-runner`

| Concern | Old Spike Wiring | New Facade Integration |
| :--- | :--- | :--- |
| **Crate Dependencies** | 3 separate crates (`blitz-host-protocol`, `blitz-host-transport`, `blitz-host-bridge`) | **1 single crate (`blitz-host`)** |
| **Global State** | `static DEBUG_SERVER: OnceLock<DebugServer>` + `static HOST_BRIDGE: Mutex<Option<HostBridge>>` | **0 static declarations in runner** (managed by `HostControl`) |
| **Startup Logic** | 15 lines of pattern matching, manual UDS printing, and mutex storage in `main()` | **1 line**: `HostControl::init_global_if_requested(...)` |
| **Frame Servicing** | 40 lines of manual mutex locking, option unwrapping, and match statements in `use_frame` | **1 call**: `HostControl::service_global_frame(...)` with `HostControl::handle_action_click(...)` |

#### Refactored Runner Code in `oxidase-native-runner/src/main.rs`:
```rust
// 1. Single dependency import
use blitz_host::HostControl;

#[oxidase::main]
fn main() {
    // 2. High-level initialization
    let is_debug_control = HostControl::init_global_if_requested(
        "oxidase-native-runner",
        env!("CARGO_PKG_VERSION"),
    );
    ...
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let is_debug_control = use_hook(HostControl::is_global_active);
    let mut live_node_handle = use_signal(|| None::<dioxus_native::NodeHandle>);
    ...
    use_frame(move |info| {
        // 3. Clean frame service
        if is_debug_control {
            if let Some(handle) = live_node_handle() {
                HostControl::service_global_frame(
                    &handle.doc(),
                    count,
                    |action_req, base_doc| {
                        HostControl::handle_action_click(action_req, base_doc, |d, nid| {
                            dioxus_native::dispatch_synthetic_click(
                                d,
                                blitz_dom::NodeId::from_u64(nid),
                                keyboard_types::Modifiers::empty(),
                            )
                        })
                    },
                );
            }
        }
    });

    rsx! {
        div {
            onmounted: move |evt| {
                live_node_handle.set(evt.downcast::<dioxus_native::NodeHandle>().cloned());
            },
            ...
        }
    }
}
```

---

## 4. Validation Actually Run

### A. Full Workspace Unit Test Suite
```bash
cargo test --manifest-path util/blitz-host/Cargo.toml -- --nocapture
```
**Results**:
- `blitz-host`: 0 unit tests (facade crate; compiles cleanly)
- `blitz_host_bridge`: 1 passed (`test_inspect_document_minimal`)
- `blitz_host_protocol`: 2 passed (`test_descriptor_serde_roundtrip`, `test_control_envelope_serde_roundtrip`)
- `blitz_host_transport`: 1 passed (`test_transport_roundtrip_server_client`)
- `live_inspect`: 1 passed (`test_live_native_runner_attach_and_inspect` - attaches to live child process with PID 53811, verifies initial button text, dispatches click, settles 2 frames, verifies `"Clicked 1 times"`, dispatches second click with `settle_until`, verifies `"Clicked 2 times"`)
- **Total**: 5 passed; 0 failed.

### B. Canonical CLI Verification
Tested against the live running interactive host:
1. `blitz-host --help`: Verified top-level help displays subcommands (`inspect`, `click`) and examples.
2. `blitz-host inspect --help`: Verified subcommand help displays `--json` and descriptor options.
3. `blitz-host click --help`: Verified subcommand help displays `<NODE_ID>` and auto-settle documentation.
4. `blitz-host inspect`: Verified live inspection connected to PID 54270 and extracted all 66 nodes and layout rects.
5. `blitz-host click 4294967402`: Verified synthetic click dispatched and auto-settled 2 VSync frames.
6. `blitz-host inspect` (re-inspect): Verified button text updated to `"Clicked 1 times"`.
7. `blitz-host click 4294967402` (second click): Verified second click dispatched and settled.
8. `blitz-host inspect` (second re-inspect): Verified button text updated to `"Clicked 2 times"`.

---

## 5. What Remains Split vs. What is Unified

- **Unified Surface (`blitz-host`)**:
  - User-facing crate dependency: `blitz-host`
  - Canonical CLI tool: `blitz-host`
  - Client interface: `blitz_host::DebugClient`
  - Host integration interface: `blitz_host::HostControl`
- **Internal Domain Split**:
  - `blitz-host-protocol` remains completely independent with 0 heavy dependencies for wire protocol purity.
  - `blitz-host-transport` remains a decoupled UDS transport layer.
  - `blitz-host-bridge` remains focused on UI-thread BaseDocument traversal and settle queues.

---

## 6. What Is Still NOT Solved (Honest Boundaries)

1. **Not Universal Upstream Dioxus Support**:
   `blitz-host` is an independent project. It is **not** part of official `DioxusLabs/dioxus` or official upstream `blitz`. Third-party applications cannot magically use `blitz-host` without adding the dependency and attaching `HostControl`.
2. **Mounted Document Capture**:
   In `dioxus-native`, `BaseDocument` is owned by the native window event loop, and components only receive document references via `NodeHandle` on mounted RSX nodes. Therefore, hosts must still provide an `onmounted` capture point or an ambient handle until upstream Blitz exposes a first-class window-level document getter.
3. **Deferred Input Matrix**:
   Keyboard input sequences, pointer movement/hover cascades, and GPU framebuffer capture remain intentionally deferred to subsequent milestones.
