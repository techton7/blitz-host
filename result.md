# Result: `blitz-host` Act + Settle Vertical Slice

## 1. Executive Summary

We have extended and runtime-proven the second vertical slice of `blitz-host`: **UI-Thread Action Execution (`Click`) and Deterministic Frame-Settle Synchronization (`Settle`)**.

This enables external test runners and agent tools to connect to a live running Blitz desktop window, inspect semantic state, dispatch typed UI actions directly through the native event engine, wait deterministically for frame settlement, and verify the resulting UI state mutation.

All requirements and user guidance have been strictly fulfilled:
1. **Definitive Proof is Observed State Change**: Success is validated not merely by waiting `frames=2`, but by querying the live Blitz semantic DOM tree and observing the button text transition from `"Click to Test Event"` to `"Clicked 1 times"` (and subsequently to `"Clicked 2 times"`).
2. **Reuse Existing Native Event Plumbing**: Instead of hacking an ad-hoc event bypass directly calling `Runtime::current().handle_event` with raw `data-dioxus-id`, the implementation reuses `dioxus-native-dom`'s native event pipeline (`synthetic_click_event`, `dispatch_synthetic_click`, and `NodeHandle::click`).
3. **Minimal Protocol Surface**: Kept strictly focused to `Act(ActionRequest::Click { node_id })` and `Settle(SettleRequest { frames })`. Focus, hover, key sequences, and generalized matrix controls are deferred.

---

## 2. Architecture & Layer Roles

The 3-crate split in `util/blitz-host/crates/` cleanly isolates domain responsibilities:

```text
util/blitz-host/
├── Cargo.toml
├── ROADMAP.md
├── instruction.md
├── result.md
└── crates/
    ├── blitz-host-protocol/      # Pure typed JSON request/response schema (zero external runtime deps)
    ├── blitz-host-transport/     # Unix Domain Socket transport, discovery, client, and test harness
    └── blitz-host-bridge/        # UI-thread bridge servicing inspect, act, and frame-settle queues
```

### A. `blitz-host-protocol`
- **Minimal Action Surface**:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(tag = "action", rename_all = "camelCase")]
  pub enum ActionRequest {
      Click { node_id: u64 },
  }

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ActionResponse {
      pub success: bool,
      pub node_id: u64,
      pub message: Option<String>,
  }
  ```
- **Frame-Settle Surface**:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct SettleRequest {
      pub frames: u32,
  }

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct SettleResponse {
      pub settled: bool,
      pub frames_waited: u32,
      pub current_frame: u64,
  }
  ```
- **Envelope Variants**:
  - `ControlRequest`: `Inspect(InspectRequest)`, `Act(ActionRequest)`, `Settle(SettleRequest)`.
  - `ControlResponse`: `InspectSuccess(InspectResponse)`, `ActionSuccess(ActionResponse)`, `SettleSuccess(SettleResponse)`, `Error(String)`.

### B. Native Event Plumbing (`dioxus-native-dom`)
Located in `blitz/packages/dioxus-native-dom/src/events.rs`:
- Added `dispatch_synthetic_click(doc: &BaseDocument, node_id: NodeId, modifiers: Modifiers) -> bool`:
  - Locates the target node in `BaseDocument`.
  - Walks up the DOM parent hierarchy if needed to find the nearest element associated with a Dioxus `ElementId`.
  - Invokes `dioxus-native-dom`'s own `synthetic_click_event(target_node, modifiers)` helper to construct full native pointer/mouse event data.
  - Packages it into `PlatformEventData` and delivers it through the active Dioxus runtime handle:
    `dioxus_core::Runtime::current().handle_event("click", dx_event, dioxus_id)`.
- Added `NodeHandle::click(&self) -> bool` convenience method.

### C. `blitz-host-bridge`
Located in `blitz-host-bridge/src/bridge.rs`:
- Manages an internal pending settle queue: `Vec<PendingSettle>`.
- Provides `poll_and_service_with(doc, current_frame, dispatch_action)`:
  1. Drains incoming `ControlRequest`s from the UDS transport channel.
  2. For `Inspect`: runs `inspect_document(doc, req)` on the UI thread and responds immediately.
  3. For `Act(action)`: invokes the runner-provided `dispatch_action(&action, doc)` closure on the UI thread, returning `ActionSuccess` or `Error`.
  4. For `Settle(req)`: pushes a `PendingSettle` with target frame `current_frame + req.frames`.
  5. On every frame turn: checks pending settles where `current_frame >= target_frame`, fulfills them with `SettleResponse { settled: true, frames_waited, current_frame }`, and sends responses back over the socket.

### D. `blitz-host-transport`
Located in `blitz-host-transport/src/client.rs`:
- High-level client API:
  - `client.act(ActionRequest)` / `client.click(node_id)`
  - `client.settle(frames)`
  - `client.settle_until(timeout, predicate)`: repeatedly settles 1 frame and re-inspects until the target predicate returns true or timeout expires.

---

## 3. Runner Integration (`oxidase-native-runner`)

Integrated in `util/oxidase/crates/oxidase-native-runner/src/main.rs`:
- **State**: Reactive `click_count: Signal<u32>`.
- **Target Button**:
  ```rust
  button {
      id: "test-interaction-button",
      onclick: move |_| {
          let new_count = click_count() + 1;
          click_count.set(new_count);
          println!("[oxidase-native-runner] User clicked interaction button! Click count: {} (Frame: {})", new_count, frame_count());
      },
      "{button_label}"
  }
  ```
  Where `button_label` renders `"Click to Test Event"` when `click_count == 0`, and `"Clicked {} times"` thereafter.
- **Hosted Frame Service**:
  Inside `use_frame`:
  ```rust
  let serviced = bridge.poll_and_service_with(
      &doc_ref,
      count,
      |action_req, base_doc| match action_req {
          blitz_host_protocol::ActionRequest::Click { node_id } => {
              let nid = blitz_dom::NodeId::from_u64(*node_id);
              let success = dioxus_native::dispatch_synthetic_click(
                  base_doc,
                  nid,
                  keyboard_types::Modifiers::empty(),
              );
              if success {
                  Ok(blitz_host_protocol::ActionResponse {
                      success: true,
                      node_id: *node_id,
                      message: Some(format!("Dispatched synthetic click to node #{node_id}")),
                  })
              } else {
                  Err(format!("Node #{node_id} or active listener not found in document"))
              }
          }
      },
  );
  ```

---

## 4. Live Runtime Proof Evidence

The automated test `tests/live_inspect.rs` spawns `oxidase-native-runner --debug-control`, attaches to the live Unix Domain Socket, and runs through the complete end-to-end lifecycle:

### Test Execution Command
```bash
cargo test --manifest-path util/blitz-host/Cargo.toml --test live_inspect -- --nocapture
```

### Raw Test Output
```text
running 1 test
Starting oxidase-native-runner in --debug-control mode...
Spawned child process with PID: 9544
Successfully connected to live host on attempt #5
Host Descriptor:
  • PID       : 9544
  • Socket    : /var/folders/p0/yfjhhmkx7ls7p_xznxcjxj900000gn/T/blitz-host/9544-1789997960602929000.sock
  • Instance  : 9544-1789997960602929000
  • Renderer  : oxidase-native-runner
Inspect Result:
  • Document ID : 1
  • Root Node ID: 4294967297
  • Total Nodes : 66
  • Observed tags: ["#document", "html", "head", "body", "main", "div", "div", "div", "div", "div", "div", "div", "div", "div", "div", "div", "div", "p", "p", "p", "p", "p", "p", "h1", "p", "div", "div", "div", "div", "span", "div", "div", "div", "div", "div", "div", "div", "div", "div", "div", "div", "#text", "button", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text", "#text"]
  • Nodes with layout bounds: 42
  • Initial button text: Some("Click to Test Event")
Dispatching click action to button node #4294967402...
Act Response: success=true, message=Some("Dispatched synthetic click to node #4294967402")
Sending settle request for 2 frames...
Settle Response: settled=true, frames_waited=2, current_frame=5
  • Observed button text after click + settle(2): Some("Clicked 1 times")
Dispatching second click action to verify continuous reactivity...
Testing settle_until helper waiting for 'Clicked 2 times'...
  • Final button text after second click + settle_until: Some("Clicked 2 times")
=================================================================
LIVE ATTACH, ACT, SETTLE, AND STATE-CHANGE PROOF PASSED 100%!
=================================================================
test test_live_native_runner_attach_and_inspect ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.16s
```

### Full Workspace Unit Test Suite
```bash
cargo test --manifest-path util/blitz-host/Cargo.toml
```
Results:
- `blitz_host_bridge`: 1 passed
- `blitz_host_protocol`: 2 passed (descriptor serde, control envelope serde)
- `blitz_host_transport`: 1 passed (in-memory UDS roundtrip)
- `live_inspect`: 1 passed (live running Blitz window attach, act, settle, state change)
- Total: 5 passed; 0 failed.

---

## 5. Explicitly Deferred Surface

In accordance with minimal surface guidelines:
1. Keyboard typing & text entry actions (`ActionRequest::Type`, `ActionRequest::KeyPress`).
2. Mouse motion / hover / pointer gesture sequences (`PointerDown`, `PointerMove`, `PointerUp`).
3. Framebuffer / GPU screenshot capture (`CaptureRequest`).
4. Multi-window routing and multi-client broadcast.
