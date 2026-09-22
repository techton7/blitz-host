# Result: Moving `blitz-host` Integration Up to the `oxidase` Ecosystem Boundary

## 1. Executive Summary

We have successfully refactored the integration architecture so that:
1. **`blitz-host` remains an independent engine/tooling crate family**: It continues to own the typed protocol, local UDS transport, bridge inspection/dispatch logic, high-level client SDK, and CLI tool.
2. **`oxidase` owns the ergonomic developer-facing convenience boundary**: Through an optional `blitz-host` Cargo feature, `oxidase` exposes transparent debug-control lifecycle management and root component injection via `#[oxidase::main]`.
3. **Downstream consumers are 100% free of direct `blitz-host` dependencies**: Applications such as `oxidase-native-runner` now depend only on `oxidase` with `features = ["native", "blitz-host"]`. In user code, there are zero `blitz-host` imports, zero `init_if_debug(...)` invocations in `main()`, and zero `<BlitzHost>` wrappers in RSX.
4. **`dioxus-native-dom` remains completely clean**: It has zero dependencies on `blitz-host`.
5. **100% of the live runtime proof was preserved**: Verified across unit tests, doc-tests, automated integration testing (`tests/live_inspect.rs`), auto-close native execution, and live interactive CLI verification (`inspect` -> `click` -> `settle` -> mutated DOM state).

---

## 2. Whether Real `oxidase`-Side Integration Code Was Added

**Yes, real, production-ready integration code was implemented directly in `oxidase`:**

1. **`util/oxidase/crates/oxidase/Cargo.toml`**:
   - Added optional dependency:
     ```toml
     blitz-host = { git = "https://github.com/techton7/blitz-host.git", version = "0.1.0", features = ["dioxus-native"], optional = true }
     ```
   - Added Cargo feature:
     ```toml
     [features]
     blitz-host = ["native", "dep:blitz-host"]
     ```
2. **`util/oxidase/crates/oxidase/src/launch.rs`**:
   - Implemented `HostedRootWrapper`:
     - When `feature = "blitz-host"` is enabled: transparently wraps child RSX in `::blitz_host::BlitzHost { children }`.
     - When `feature = "blitz-host"` is disabled: acts as a zero-overhead pass-through rendering `children` directly.
   - Implemented `init_debug_control_if_available()`:
     - Automatically parses CLI args (`--debug-control`) and environment variables (`BLITZ_DEBUG_CONTROL=1`).
     - Initializes the debug control server and registers the host descriptor under `/tmp/blitz-host/`.
   - Implemented `is_debug_control_active()`:
     - Returns `true` if debug control is currently active in the process.
3. **`util/oxidase/crates/oxidase/src/lib.rs` & `prelude.rs`**:
   - Conditionally re-exports `blitz_host` crate when `feature = "blitz-host"` is active.
   - Re-exports `is_debug_control_active` in `prelude`.
4. **`util/oxidase/crates/oxidase-macro/src/main_macro.rs`**:
   - Updated the `#[oxidase::main]` attribute macro for native targets:
     - Automatically emits `::oxidase::launch::init_debug_control_if_available();` before launching Dioxus.
     - Wraps the root `#app {}` component in `::oxidase::launch::HostedRootWrapper { #app {} }`.
     - By isolating the `#[cfg(feature = "blitz-host")]` logic inside `oxidase::launch::HostedRootWrapper`, the proc macro avoids evaluating consumer crate feature flags, guaranteeing clean macro expansion across all host crates.
5. **`util/oxidase/crates/oxidase-native-runner` Refactored**:
   - `Cargo.toml`: Removed direct dependency on `blitz-host`. Now depends exclusively on `oxidase = { path = "../oxidase", features = ["native", "blitz-host"] }`.
   - `src/main.rs`: Purged all `blitz-host` imports, purged `blitz_host::init_if_debug(...)`, and purged `<BlitzHost>` in `App()`. The runner code is now pristine standard Dioxus Native RSX.

---

## 3. The Intended Dependency Line

The intended downstream dependency model is now fully aligned with standard Git tag releases:

### A. Downstream Application `Cargo.toml`
```toml
[dependencies]
dioxus = { version = "0.7.1", default-features = false }
oxidase = { git = "https://github.com/techton7/oxidase.git", tag = "oxidase-v0.1.4", features = ["native", "blitz-host"] }
```

### B. Downstream Application `main.rs`
```rust
use dioxus::prelude::*;

#[oxidase::main]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            h1 { "Hello Native World" }
        }
    }
}
```

### C. Runtime Activation
- By default, running `cargo run` launches the application normally with zero debug server overhead.
- Passing `--debug-control` (or setting `BLITZ_DEBUG_CONTROL=1`) automatically starts the debug control socket server, writes the host descriptor to `/tmp/blitz-host/`, and enables external agent inspection and action dispatch.

---

## 4. Why `dioxus-native-dom` Does NOT Need to Know About `blitz-host`

`dioxus-native-dom` remains completely dependency-clean with respect to `blitz-host` for three core architectural reasons:

1. **Separation of Concerns**:
   `dioxus-native-dom` is a low-level DOM / layout integration crate for Blitz. Its sole responsibility is mapping Dioxus VirtualDOM mutations into Blitz's `BaseDocument` and layout structures. It should never know about IPC protocols, Unix domain sockets, debug serialization formats, or remote agent control.
2. **Sufficiency of Existing Public APIs**:
   `dioxus-native-dom` already exports:
   - `NodeHandle`: enables downcasting mounted element references to access the underlying `BaseDocument` and `NodeId`.
   - `dispatch_synthetic_click`: allows synthetic event injection into the native event system.
   `blitz-host-bridge` consumes these public APIs without requiring any specialized, private hooks.
3. **Avoiding Dependency Pollution & Cyclic Graph Risks**:
   If `dioxus-native-dom` had a direct dependency on `blitz-host`, every consumer of Blitz and Dioxus Native would be forced to pull in socket servers, Tokio/async runtimes, and serialization crates, even when running minimal headless layout tests. Keeping `blitz-host` at the `oxidase` ecosystem boundary preserves modularity.

---

## 5. What Remains Inside `blitz-host`

The `blitz-host` workspace (`util/blitz-host/`) maintains full structural independence and consists of:

1. **`blitz-host-protocol`**:
   - Pure data contracts and serialization models (`InspectRequest`, `InspectResponse`, `ActionRequest`, `ActionResponse`, `SettleRequest`, `SettleResponse`, `HostDescriptor`).
   - Zero UI, zero Winit, and zero Blitz dependencies.
2. **`blitz-host-transport`**:
   - Local IPC engine (Unix domain sockets on macOS/Linux).
   - Owner-restricted filesystem permissions (`0o700` socket directory).
   - Length-delimited framing and client/server session handling.
3. **`blitz-host-bridge`**:
   - UI-thread bridge and document inspector.
   - Recursive DOM tree traversal, layout bounds extraction, tag and text extraction.
   - Synthetic click dispatch and frame quiescence/settling.
4. **`blitz-host` (Facade Crate)**:
   - High-level client SDK (`DebugClient`).
   - Host lifecycle controller (`HostControl`).
   - Dioxus Native integration component (`BlitzHost`).
5. **`blitz-host` (CLI Binary)**:
   - Interactive diagnostic and agent tool (`blitz-host inspect`, `blitz-host click <NODE_ID>`).
6. **`tests/live_inspect.rs`**:
   - Integration test proving live end-to-end attach, inspect, act, settle, and verification against running native binaries.

---

## 6. What Still Remains Unresolved

While the `oxidase` ecosystem boundary is fully established and working, the following limitations remain:

1. **Non-`oxidase` Dioxus Native Hosts**:
   Applications that choose *not* to use `oxidase` (i.e. using raw `dioxus::launch` without `#[oxidase::main]`) cannot benefit from zero-boilerplate injection. They must explicitly depend on `blitz-host` and place `<BlitzHost>` in their root RSX, because upstream Blitz 0.3 does not offer a global ambient document handle.
2. **Expanded Synthetic Action Primitives**:
   Currently, synthetic action dispatch is proven for primary click events. Keyboard input sequences, modifier keys, pointer move/hover states, drag-and-drop, and scroll gestures remain to be implemented in `blitz-host-bridge`.
3. **Visual Framebuffer Capture**:
   Direct readback of the Vello GPU surface to PNG/JPEG images over the socket is planned but deferred to a later milestone.
4. **Multi-Window Architectures**:
   Currently, inspection targets the primary document (`Document ID 1`). Multi-window orchestration will require extending the protocol to enumerate window IDs.

---

## 7. Validation Actually Run

### A. Full Workspace Test Suite
```bash
cargo test --manifest-path util/blitz-host/Cargo.toml -- --nocapture
```
**Results**:
- `blitz-host`: ok (0 unittests, 0 doc-tests)
- `blitz_host_bridge`: ok (1 unit test passed: `test_inspect_document_minimal`)
- `blitz_host_protocol`: ok (2 unit tests passed: `test_descriptor_serde_roundtrip`, `test_control_envelope_serde_roundtrip`)
- `blitz_host_transport`: ok (1 unit test passed: `test_transport_roundtrip_server_client`)
- `tests/live_inspect.rs`: ok (1 integration test passed in 1.52s):
  - Spawns `oxidase-native-runner` in `--debug-control` mode
  - Connects to UDS socket on attempt #1
  - Inspects live DOM (67 nodes, extracts initial button label `"Click to Test Event"`)
  - Dispatches synthetic click to button node `#4294967406`
  - Settles 2 VSync frames
  - Verifies DOM label updated to `"Clicked 1 times"`
  - Dispatches second click and validates `settle_until` (`"Clicked 2 times"`)

### B. `oxidase` Compilation & Check
```bash
cargo check --manifest-path util/oxidase/crates/oxidase/Cargo.toml --features native,blitz-host
cargo build --manifest-path util/oxidase/crates/oxidase-native-runner/Cargo.toml
```
**Results**:
- Both compiled cleanly with exit code 0.

### C. Runner Auto-Close Native Proof
```bash
cargo run --manifest-path util/oxidase/crates/oxidase-native-runner/Cargo.toml
```
**Results**:
- Native window mounted via Blitz 0.3 / Vello GPU.
- Successfully rendered 20 frames and auto-closed with exit code 0 without debug control overhead.

### D. Live Interactive CLI Verification
```bash
# Terminal 1: Launch runner in debug control mode
target/debug/oxidase-native-runner --interactive --debug-control

# Terminal 2: blitz-host CLI commands
blitz-host inspect
blitz-host click 4294967406 --settle-frames 2
blitz-host inspect
```
**Results**:
- Initial inspection: Document ID 1, 67 nodes, button text `"Click to Test Event"`.
- Click command: Dispatched synthetic click, settled 2 frames.
- Re-inspection: Verified live DOM mutation with updated button text `"Clicked 1 times"`.

---

## 8. Final Verdict

**Implemented and directionally integrated**:
1. The boundary between `blitz-host` (internal control engine/tooling) and `oxidase` (ergonomic downstream convenience layer) is clean, explicit, and materially proven in code.
2. The dependency model is truthful: downstream consumers only require `oxidase = { features = ["native", "blitz-host"] }`, while `blitz-host` remains independent.
3. `dioxus-native-dom` remains completely free of any direct `blitz-host` dependency.
4. The macro-driven transparent injection via `#[oxidase::main]` is fully functional, removing all debug boilerplate from consumer application code.
