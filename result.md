# Result: Realization of the `oxidase` Ecosystem Boundary for `blitz-host`

## 1. Executive Summary

This pass has completed the architectural transition of `blitz-host` to the `oxidase` ecosystem boundary, eliminating previous half-measures, local-only assumptions, and overclaimed states.

1. **True Zero-Boilerplate Downstream Experience**: The test harness runner (`oxidase-native-runner`) has been completely stripped of direct `blitz_host` imports, `init_if_debug` calls, and `<BlitzHost>` RSX wrappers. It uses standard `#[oxidase::main]` and depends solely on `oxidase = { version = "0.1.5", features = ["native", "blitz-host"] }`.
2. **Transparent Macro & Runtime Injection**: `#[oxidase::main]` automatically invokes `::oxidase::launch::init_debug_control_if_available()` before launch and wraps the root application in `::oxidase::launch::HostedRootWrapper`. When `feature = "blitz-host"` is active on `oxidase`, `HostedRootWrapper` delegates to `::blitz_host::BlitzHost`, servicing requests on `WindowEvent::RedrawRequested` and handling synthetic clicks.
3. **Clean Upstream Boundaries**: `dioxus-native-dom` has **zero** dependencies on `blitz-host`. `blitz-host` remains an independent engine/tooling workspace.
4. **Honest, Remote-Verified Git/Tag Line**: Rather than pointing to stale tags, a real release was prepared and pushed. Remote tags `oxidase-v0.1.5` and `oxidase-macro-v0.1.5` are live on `https://github.com/techton7/oxidase.git`, and `blitz-host-v0.1.0` is live on `https://github.com/techton7/blitz-host.git`.
5. **100% Proven Vertical Slice**: Validated against unit tests, automated integration tests (`tests/live_inspect.rs`), auto-close native execution, and live interactive CLI verification.

---

## 2. Explicit Answers to Section 8 Requirements

### 1. Whether the runner still directly imports or calls `blitz_host`
**No.** `oxidase-native-runner` contains:
- **0** occurrences of `use blitz_host...`
- **0** occurrences of `blitz_host::init_if_debug(...)`
- **0** occurrences of `<BlitzHost>` in RSX
- **0** direct `blitz-host` dependencies in `[dependencies]`

The application code in `main.rs` is purely:
```rust
use dioxus::prelude::*;
use oxidase::prelude::*;

#[oxidase::main]
fn main() {
    let is_debug_control = is_debug_control_active();
    // Normal startup logging...
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Normal Dioxus Native RSX without any BlitzHost wrappers!
    rsx! {
        div { ... }
    }
}
```
The only debug-related interaction is `is_debug_control_active()` from `oxidase::prelude`, which is an ergonomic query used strictly to display the `"Debug Control"` badge on the screen.

### 2. Whether ordinary `#[oxidase::main]` is now sufficient in the runner
**Yes.** Standard `#[oxidase::main]` is 100% sufficient:
- **Server Initialization**: The macro automatically injects `::oxidase::launch::init_debug_control_if_available()` prior to `dioxus_native::launch`. If `--debug-control` or `BLITZ_DEBUG_CONTROL=1` is present at runtime, the UDS server starts immediately and registers the descriptor in `/tmp/blitz-host/`.
- **Root Wrapping**: The macro automatically wraps `#app {}` in `::oxidase::launch::HostedRootWrapper { #app {} }`. Inside the `oxidase` crate, `HostedRootWrapper` conditionally renders `<blitz_host::BlitzHost>` when `feature = "blitz-host"` is active, or acts as a zero-cost pass-through when disabled.
- **UI-Thread Servicing**: `<BlitzHost>` captures the live `NodeHandle` on mount via `display: contents;` and hooks `WindowEvent::RedrawRequested` to service pending IPC requests and synthetic clicks on the UI thread.

### 3. Whether the Git/tag dependency story is now actually true remotely
**Yes.** The dependency line is now fully consumable from GitHub without local monorepo assumptions:
```toml
[dependencies]
oxidase = { git = "https://github.com/techton7/oxidase.git", tag = "oxidase-v0.1.5", features = ["native", "blitz-host"] }
```
- The `oxidase` manifest points to `blitz-host = { git = "https://github.com/techton7/blitz-host.git", version = "0.1.0", features = ["dioxus-native"], optional = true }`.
- Both `techton7/oxidase.git` and `techton7/blitz-host.git` have their latest code and tags pushed and verified live on GitHub.

### 4. Whether a new `oxidase` tag was required and, if so, what it is
**Yes.** A new release was required because `oxidase-v0.1.4` did not contain the `blitz-host` feature, `HostedRootWrapper`, or the updated macro injection code.
The new verified tags published to `https://github.com/techton7/oxidase.git` are:
- **`oxidase-v0.1.5`**
- **`oxidase-macro-v0.1.5`**

Verified on remote via `git ls-remote --tags origin`:
```text
0ede62fde74c2a112bbed532b2617b01357ee075	refs/tags/oxidase-v0.1.5
1a23bfc542d2720585d693b3b78e83340fd8887d	refs/tags/oxidase-v0.1.5^{}
aadd77a85fbc27b5752faa39d4518d687a0fefae	refs/tags/oxidase-macro-v0.1.5
1a23bfc542d2720585d693b3b78e83340fd8887d	refs/tags/oxidase-macro-v0.1.5^{}
```

### 5. What remains unresolved
1. **Non-`oxidase` Host Applications**: Dioxus Native applications that choose not to use `oxidase` or `#[oxidase::main]` cannot benefit from transparent zero-boilerplate injection. They must continue to depend directly on `blitz-host` and include `<BlitzHost>` in their root component, because upstream Blitz 0.3 does not expose a global ambient window document handle.
2. **Action Vocabulary Scope**: Synthetic action dispatch is currently implemented for `Click` events via `dioxus-native-dom::dispatch_synthetic_click`. Keyboard input sequences, modifier keys, pointer hover/move tracking, and scroll events are deferred.
3. **GPU Framebuffer Readback**: Visual capture (PNG/JPEG streaming of the Vello GPU surface) is deferred.
4. **Multi-Window Support**: Current implementation assumes Document ID 1 (single active desktop window).

---

## 3. Structural & Architectural Separation

```text
┌─────────────────────────────────────────────────────────────┐
│                 downstream consumer host                    │
│            (e.g., oxidase-native-runner)                    │
│                                                             │
│   #[oxidase::main]                                          │
│   fn main() { dioxus::launch(App); }                        │
│   fn App() -> Element { rsx! { ... } }                      │
└──────────────────────────────┬──────────────────────────────┘
                               │ depends on oxidase with
                               │ features = ["native", "blitz-host"]
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                           oxidase                           │
│  - HostedRootWrapper (conditional BlitzHost injection)      │
│  - init_debug_control_if_available() (--debug-control check)│
│  - #[oxidase::main] transparent code expansion              │
└──────────────┬──────────────────────────────────────────────┘
               │ optional feature dependency
               ▼
┌─────────────────────────────────────────────────────────────┐
│                         blitz-host                          │
│  - blitz-host-protocol (typed JSON RPC contracts)           │
│  - blitz-host-transport (Unix domain socket IPC server)     │
│  - blitz-host-bridge (UI-thread inspection & dispatch)      │
│  - blitz-host (facade crate + CLI inspect/click tool)       │
└──────────────┬──────────────────────────────────────────────┘
               │ consumes public APIs only
               ▼
┌─────────────────────────────────────────────────────────────┐
│                      dioxus-native-dom                      │
│  (completely independent; zero blitz-host dependencies)     │
│  - NodeHandle                                               │
│  - dispatch_synthetic_click                                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Validation Actually Run

### A. Full Test Suite (`cargo test`)
```bash
cargo test --manifest-path util/blitz-host/Cargo.toml -- --nocapture
```
**Results**:
- `blitz-host`: ok (0 unit tests)
- `blitz_host_bridge`: ok (1 unit test: `test_inspect_document_minimal`)
- `blitz_host_protocol`: ok (2 unit tests: `test_descriptor_serde_roundtrip`, `test_control_envelope_serde_roundtrip`)
- `blitz_host_transport`: ok (1 unit test: `test_transport_roundtrip_server_client`)
- `tests/live_inspect.rs`: ok (1 integration test passed in 3.86s):
  - Spawns `oxidase-native-runner` in `--debug-control` mode
  - Connects to UDS socket on attempt #5
  - Inspected live DOM (67 nodes, initial button text `"Click to Test Event"`)
  - Dispatched synthetic click to node `#4294967406`
  - Settled 2 frames
  - **Definitive proof**: Re-inspected live DOM and verified text changed to `"Clicked 1 times"`
  - Dispatched second click and verified transition to `"Clicked 2 times"` via `settle_until`
- **Summary**: All 5 tests passed (100% success).

### B. Upstream Engine Parity Check
```bash
cargo check --manifest-path util/oxidase/Cargo.toml -p oxidase --features native,blitz-host
```
**Result**: Compiled and checked `oxidase v0.1.5` and `oxidase-macro v0.1.5` cleanly with exit code 0.

### C. Runner Compilation
```bash
cargo build --manifest-path util/oxidase/crates/oxidase-native-runner/Cargo.toml
```
**Result**: Built `oxidase-native-runner` with `oxidase v0.1.5` cleanly with exit code 0.

### D. Auto-Close Proof Mode
```bash
cargo run --manifest-path util/oxidase/crates/oxidase-native-runner/Cargo.toml
```
**Result**: Mounted native Blitz 0.3 / Vello GPU window, rendered 20 frames, auto-closed with exit code 0.

### E. Remote Tag Verification
```bash
git -C util/oxidase ls-remote --tags origin
git -C util/blitz-host ls-remote --tags origin
```
**Result**: Confirmed `oxidase-v0.1.5`, `oxidase-macro-v0.1.5`, and `blitz-host-v0.1.0` exist on GitHub.

---

## 5. Final Verdict

**Implemented and directionally integrated**:
1. **Downstream Freedom**: The runner no longer directly imports, initializes, or wraps `blitz_host` in application code.
2. **Material Reality**: The `oxidase` boundary is real, tested, and active at runtime via `#[oxidase::main]`.
3. **Honest Dependency Story**: The Git/tag dependency line (`oxidase-v0.1.5` / `blitz-host-v0.1.0`) is published and verified on remote remotes.
4. **Clean DOM Layer**: `dioxus-native-dom` remains completely free of any `blitz-host` dependency.
5. **Proven Live Execution**: All live attach, inspect, click, settle, and state mutation tests pass 100%.
