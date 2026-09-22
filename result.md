# Result: `blitz-host` One-Line Debug Injection Model & Host Facade Refactor

## 1. Executive Summary

We have advanced `blitz-host` from an internal set of plumbing primitives into a deliberate, near one-line host debug injection model for Dioxus Native desktop applications.

The host-side integration surface in `oxidase-native-runner` was refactored so that:
1. **Low-level bridge orchestration is completely hidden**: All manual `NodeHandle` state management, `onmounted` downcasting, per-frame `service_global_frame` invocations, and `dispatch_synthetic_click` closure wiring were removed from the runner's animation loop and UI code.
2. **Declarative Root Component (`<BlitzHost>`)**: An ergonomic root wrapper component was added to `blitz-host` under an optional `dioxus-native` feature. `<BlitzHost>` mounts an invisible container (`display: contents;`) to capture the live window's `NodeHandle`, listens to native `WindowEvent::RedrawRequested` events via `dioxus_native::use_window_event`, and automatically handles action dispatches (e.g. synthetic clicks) on the main UI thread.
3. **100% Proven Vertical Slice Preserved**: Validated against both automated test suites (`cargo test`, `live_inspect`) and live interactive sessions with the `blitz-host` CLI tool (`inspect`, `click`, `settle`, and button state mutation).

---

## 2. The New Host Integration Surface

### A. Facade Crate API

In `util/blitz-host/crates/blitz-host`:
```rust
// Feature: dioxus-native
pub use dioxus::BlitzHost;

// Core Host Control
pub use host::{init_if_debug, init_if_debug_default, HostControl};

// Prelude
pub mod prelude {
    pub use crate::host::{init_if_debug, init_if_debug_default, HostControl};
    pub use crate::client::DebugClient;
    pub use crate::protocol::{ActionRequest, ActionResponse, InspectRequest, InspectResponse, SettleRequest, SettleResponse};
    #[cfg(feature = "dioxus-native")]
    pub use crate::dioxus::BlitzHost;
}
```

### B. Consumer Usage Pattern

For any Dioxus Native host wanting debug control:

```rust
use blitz_host::prelude::*;

#[oxidase::main]
fn main() {
    // 1. Optional explicit initialization with app metadata
    blitz_host::init_if_debug("my-app", env!("CARGO_PKG_VERSION"));

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // 2. Wrap root UI in BlitzHost
        BlitzHost {
            MainAppContent {}
        }
    }
}
```

If `blitz_host::init_if_debug(...)` is omitted in `main()`, `<BlitzHost>` automatically performs fallback initialization using the current process executable name when `--debug-control` or `BLITZ_DEBUG_CONTROL=1` is detected. When debug control is not active, `<BlitzHost>` simply renders `children` with zero layout interference and zero runtime overhead.

---

## 3. Did We Achieve True One-Line Injection?

### Honest Verdict: Smallest Honest Equivalent (2-Point Bootstrap or 1-Point Wrapper)

We achieved the **smallest honest equivalent**:
1. `blitz_host::init_if_debug(...)` in `main()` (for explicit logging of app name and version)
2. `BlitzHost { ... }` wrapping the root component in RSX

Alternatively, if default process name logging is acceptable, wrapping with `<BlitzHost>` is literally a **single component wrapper entrypoint**.

### Why True Zero-Code Ambient Injection is Currently Impossible in Blitz
In `dioxus-native` (Blitz 0.3), the live window's `BaseDocument` (Document ID 1) is owned by the Winit event loop. Upstream Blitz does not currently expose a global ambient window document getter; components can only obtain a live document reference via `NodeHandle`, which `dioxus-native` delivers to mounted elements via `onmounted` events.

Because `blitz-host` is an independent crate and cannot unilaterally rewrite the Dioxus VirtualDOM compiler, `<BlitzHost>` uses a CSS `display: contents;` wrapper element to capture `NodeHandle` on mount and registers a `WindowEvent::RedrawRequested` listener to poll the control plane on every native frame.

---

## 4. Runner-Side Glue: Removed vs. Remaining

### A. What Was Removed from `oxidase-native-runner`

1. **`live_node_handle` Signal**:
   - `let mut live_node_handle = use_signal(|| None::<dioxus_native::NodeHandle>);` completely deleted.
2. **Manual `onmounted` Downcast**:
   - `onmounted: move |evt| { live_node_handle.set(evt.downcast::<dioxus_native::NodeHandle>().cloned()); }` deleted from the application's root container.
3. **Manual Frame Servicing in Animation Loops**:
   - 25 lines of `HostControl::service_global_frame` calls inside `use_frame(move |info| { ... })` deleted. `use_frame` now contains only pure application animation logic.
4. **Action Dispatch Glue**:
   - The manual closure calling `dioxus_native::dispatch_synthetic_click(...)` deleted.
5. **Crate Dependencies**:
   - `keyboard-types` removed from `oxidase-native-runner/Cargo.toml`.
   - `blitz_dom::NodeId` and `keyboard_types::Modifiers` eliminated from `main.rs`.

### B. What Runner-Side Glue Still Remains and Why

1. **`blitz_host::init_if_debug("oxidase-native-runner", env!("CARGO_PKG_VERSION"));` in `main()`**:
   - **Why**: Allows the runner to explicitly announce its renderer name, version, PID, and UDS socket location in terminal output before the Winit window opens.
2. **`BlitzHost { ... }` in `rsx!`**:
   - **Why**: Houses the `onmounted` capture element and `use_window_event(RedrawRequested)` hook that bridges the Dioxus VirtualDOM to the native `BaseDocument`.
3. **`use_hook(HostControl::is_global_active)` in `App()`**:
   - **Why**: Purely cosmetic. Used only to render the `"Debug Control"` badge and footer proof checklist in the runner UI. Has zero functional role in servicing requests.

---

## 5. Validation Actually Run

### A. Full Test Suite (`cargo test`)
Command:
```bash
cargo test --manifest-path util/blitz-host/Cargo.toml -- --nocapture
```
**Results**:
- `blitz-host`: Compiles cleanly; 0 unit tests.
- `blitz_host_bridge`: 1 test passed (`test_inspect_document_minimal`).
- `blitz_host_protocol`: 2 tests passed (`test_descriptor_serde_roundtrip`, `test_control_envelope_serde_roundtrip`).
- `blitz_host_transport`: 1 test passed (`test_transport_roundtrip_server_client`).
- `tests/live_inspect.rs`: 1 integration test passed in 2.02s:
  - Spawns `oxidase-native-runner --debug-control`
  - Attaches to live UDS socket
  - Inspects live DOM (Document ID 1, 66 nodes, extracts initial button label `"Click to Test Event"`)
  - Dispatches synthetic click to button node
  - Settles 2 VSync frames
  - Verifies dynamic label update (`"Clicked 1 times"`)
  - Dispatches second click and validates `settle_until` (`"Clicked 2 times"`)
- **Summary**: All 5 tests passed (100% success).

### B. Auto-Close Proof Mode
Command:
```bash
cargo run --manifest-path util/oxidase/crates/oxidase-native-runner/Cargo.toml
```
**Results**:
- Window mounted via Blitz 0.3 / Vello GPU.
- Verified `<BlitzHost>` renders cleanly without errors when debug control is inactive.
- Successfully executed 20 real frames and exited with code 0.

### C. Live Interactive CLI Verification
Command:
```bash
# Terminal 1: Background Runner
util/oxidase/crates/oxidase-native-runner/target/debug/oxidase-native-runner --interactive --debug-control

# Terminal 2: blitz-host CLI
util/blitz-host/target/debug/blitz-host inspect
util/blitz-host/target/debug/blitz-host click 4294967406 --settle-frames 2
util/blitz-host/target/debug/blitz-host inspect
```
**Observed Output**:
- First `inspect`: Connected to PID 68305. Document ID 1, Root ID 4294967297, 67 nodes. Button node `#4294967406` with text `"Click to Test Event"`.
- `click 4294967406 --settle-frames 2`: Dispatched click action (`success=true`), waited 2 frames, settled at current frame 8630.
- Second `inspect`: Button text `#4294967424` updated to `"Clicked 1 times"`.

---

## 6. Remaining Limitations

1. **Opt-in Dependency**:
   Host applications must add `blitz-host = { version = "...", features = ["dioxus-native"] }` and wrap their root component in `<BlitzHost>`. This is not an uninvited system-level hook.
2. **VirtualDOM Boundary**:
   Capturing the live `BaseDocument` requires at least one mounted element (`<BlitzHost>`). If the Dioxus component tree never mounts (e.g. fatal panic during setup), debug control cannot inspect the live document.
3. **Deferred Actions**:
   Synthetic keyboard sequences, pointer movement/hover cascades, and GPU framebuffer capture remain deferred to subsequent milestones.
