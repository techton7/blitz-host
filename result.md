# Result: Implementation and Verification of Blitz Host Bounded Core Keyboard Lane

## 1. Executive Summary

This phase implemented and verified the bounded core keyboard action lane for `blitz-host`, enabling automated test runners, AI agents, and developers to drive realistic keyboard workflows against live Blitz desktop applications.

1. **Protocol & Wire Vocabulary (`crates/blitz-host-protocol`)**:
   - Added `KeyModifiers { shift: bool, ctrl: bool, alt: bool, meta: bool }` with bitflags-style helpers, canonical masks (`NONE`, `SHIFT`, `CTRL`, `ALT`, `META`), and platform-aware `action_modifier()`.
   - Added `ActionRequest::Key { window_id: Option<u64>, node_id: Option<u64>, key: String, modifiers: Option<KeyModifiers> }`.
2. **Upstream Engine Semantic Fixes (`blitz-dom` & `dioxus-native-dom`)**:
   - **`blitz-dom/src/document.rs`**: Updated `focus_next_node()` and `focus_prev_node()` to fall back to `root_node_id` when no node currently holds focus. Added `active_focus_node_id()` to expose the explicit focused element ID without falling back to document root.
   - **`blitz-dom/src/events/keyboard.rs`**: Handled `Key::Escape` to invoke `doc.clear_focus()`.
   - **`blitz-dom/src/node/text.rs`**: Added macOS fallback branches for `Key::ArrowLeft`, `Key::ArrowRight`, `Key::ArrowUp`, `Key::ArrowDown`, `Key::Backspace`, and `Key::Delete` when `action_mod` is not set, eliminating macOS Cocoa selector dependency in direct event dispatch.
   - **`dioxus-native-dom/src/events.rs`**:
     - Implemented `parse_key_str(&str) -> (Key, Code, Modifiers)` supporting compound key strings (`"Shift+Tab"`, `"Cmd+A"`, `"Ctrl+A"`) and core key names.
     - Implemented `dispatch_synthetic_key(doc, node_id, key_str, modifiers) -> Result<NodeId, String>`:
       - Drives `Tab` / `Shift+Tab` focus cycling and emits Dioxus `focus`, `focusin`, and `blur` events.
       - Drives `Escape` focus clearing and emits Dioxus `blur`.
       - Drives `Enter` / `Space` button activation via direct click dispatch.
       - Dispatches `UiEvent::KeyDown` / `UiEvent::KeyUp` to `BaseDocument` via `doc.handle_ui_event`.
       - Propagates updated text values to Dioxus listeners via `NativeFormData` (`input` / `change` events).
       - Dispatches Dioxus `keydown` / `keyup` events to the active VirtualDom runtime.
3. **Transport & Client Surface (`crates/blitz-host-transport`)**:
   - `DebugClient::key(&mut self, window_id: Option<u64>, node_id: Option<u64>, key: impl Into<String>) -> io::Result<ActionResponse>`
   - `DebugClient::key_with_modifiers(...)`
   - Convenience helpers: `tab()`, `shift_tab()`, `enter()`, `space()`, `escape()`, `backspace(node_id)`, `delete(node_id)`, `select_all(node_id)`.
4. **CLI Subcommand (`crates/blitz-host`)**:
   - Added `blitz-host key <KEY> [OPTIONS] [DESCRIPTOR_PATH]`.
   - Flags: `--node <ID>`, `--shift`, `--ctrl`, `--alt`, `--meta` / `--cmd`, `--pid <PID>`, `--window <ID>`.
   - Automatically synchronizes 2 VSync frames post-dispatch to settle reactive state changes before exiting.
5. **Multi-Stage Verification Passed 100%**:
   - **Stage 1 (Protocol)**: `cargo test -p blitz-host-protocol` verified `KeyModifiers` and `ActionRequest::Key` serialization roundtrip.
   - **Stage 2 (Headless Semantics)**: `cargo test --manifest-path blitz/Cargo.toml -p dioxus-native-dom test_synthetic_keyboard_events` verified full end-to-end headless lifecycle across Tab/Shift+Tab focus traversal, Enter/Space activation, typing + Backspace editing, Cmd/Ctrl+A select all and overwrite, and Escape focus clearing.
   - **Stage 3 (Live Native E2E)**: `tests/live_inspect.rs` verified full live keyboard workflow against running native host (`oxidase-native-runner` with Winit 0.31 / Vello GPU VSync), confirming real inspect-observed state mutations after settle.
   - **Interactive CLI Proof**: Verified against running `cross_host --features blitz-host` (PID 79487) using `blitz-host key` commands for button activation (`Enter`, `Space`), focus traversal (`Shift+Tab`), typing (`H`, `i`), deleting (`Backspace`), selecting all and overwriting (`Cmd+A` + `Z`), and clearing focus (`Escape`).

---

## 2. Keyboard Action Surface Added

### 2.1 Protocol Vocabulary (`crates/blitz-host-protocol`)
In `crates/blitz-host-protocol/src/lib.rs`:
```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyModifiers {
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub meta: bool,
}

pub enum ActionRequest {
    ...
    Key {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modifiers: Option<KeyModifiers>,
    },
}
```

### 2.2 Host & Dioxus Integration (`crates/blitz-host`)
In `crates/blitz-host/src/host.rs` and `crates/blitz-host/src/dioxus.rs`:
- `HostControl::handle_action` accepts `key_fn: K` and delegates `ActionRequest::Key` to `dioxus_native::dispatch_synthetic_key`.
- `BlitzHost` automatically hooks into `WindowEvent::RedrawRequested` to service pending key requests before frame render.

### 2.3 Client API (`crates/blitz-host-transport`)
In `crates/blitz-host-transport/src/client.rs`:
- `client.key(window_id, node_id, key)`
- `client.key_with_modifiers(window_id, node_id, key, modifiers)`
- `client.tab()`
- `client.shift_tab()`
- `client.enter()`
- `client.space()`
- `client.escape()`
- `client.backspace(node_id)`
- `client.delete(node_id)`
- `client.select_all(node_id)`

### 2.4 CLI Surface (`crates/blitz-host`)
```bash
blitz-host key <KEY> [OPTIONS] [DESCRIPTOR_PATH]

OPTIONS:
    --node <NODE_ID>     Target specific node ID (optional, defaults to focused element)
    --shift              Hold Shift modifier
    --ctrl               Hold Ctrl modifier
    --alt                Hold Alt/Option modifier
    --meta               Hold Meta/Cmd modifier
    --pid <PID>          Target specific host process by OS PID
    --window <ID>        Target specific window ID
```

---

## 3. How Focus Traversal Was Proven

Focus traversal via `Tab` and `Shift+Tab` was proven across two levels:
1. **Headless (`dioxus-native-dom::test_synthetic_keyboard_events`)**:
   - Initial state had no focused node.
   - `dispatch_synthetic_key(..., None, "Tab", ...)` navigated focus to `input#key-input` (`tab1 == input_id`).
   - Dispatched `onfocus` handler, updating reactive signal `input_focused == true`.
   - Second `Tab` moved focus to `button#key-button` (`tab2 == btn_id`), firing `input.onblur` (`input_focused == false`) and `btn.onfocus` (`btn_focused == true`).
   - `Shift+Tab` reversed focus back to `input#key-input` (`stab == input_id`), firing `btn.onblur` and `input.onfocus`.
2. **Live Native Host (`tests/live_inspect.rs` & `cross_host`)**:
   - Button held focus (`#4294967417`).
   - Dispatched `client.shift_tab()` (or `blitz-host key Shift+Tab --pid 79487`).
   - Live semantic inspect confirmed `focused_node_id` shifted to `Some(4294967413)` (`input#test-input`).
   - `cross_host` stdout emitted: `[cross_host] Input focused!`.

---

## 4. How Submit / Activation Was Proven

Button activation via `Enter` and `Space` was proven by observing real reactive state mutation:
1. **Headless (`dioxus-native-dom::test_synthetic_keyboard_events`)**:
   - With `button#key-button` focused, dispatched `Enter` -> `clicks` incremented from 0 to 1.
   - Dispatched `Space` -> `clicks` incremented from 1 to 2.
2. **Live Native Host (`tests/live_inspect.rs` & `cross_host`)**:
   - Button began with label `"Clicked 2 times"`.
   - Dispatched `client.enter()` (and `blitz-host key Enter --pid 79487`).
   - After settle(2), live semantic inspect confirmed label updated to `"Clicked 3 times"`.
   - Dispatched `client.space()` (and `blitz-host key Space --pid 79487`).
   - After settle(2), live semantic inspect confirmed label updated to `"Clicked 4 times"`.
   - `cross_host` stdout emitted: `[cross_host] Button clicked! Count: 1`, `Count: 2`.

---

## 5. How Editing and Navigation Were Proven

Text editing was proven by observing input text and reactive consumer elements:
1. **Headless (`dioxus-native-dom::test_synthetic_keyboard_events`)**:
   - Dispatched key `"a"` to `input_id` -> `typed` signal updated to `"a"`.
   - Dispatched key `"c"` -> `typed` signal updated to `"ac"`.
   - Dispatched key `"Backspace"` -> deleted `"c"`, `typed` signal updated to `"a"`.
2. **Live Native Host (`tests/live_inspect.rs` & `cross_host`)**:
   - Dispatched key `"H"` then `"i"` to `input#test-input` (`#4294967413`).
   - Live semantic inspect confirmed `p#typed-text` rendered: `"Typed: Hi"`.
   - Dispatched `Backspace` to `input#test-input`.
   - Live semantic inspect confirmed `p#typed-text` rendered: `"Typed: H"`.
   - `cross_host` stdout emitted: `[cross_host] Input value changed: H`, `Hi`, `H`.

---

## 6. How the Modifier Sentinel (`Ctrl/Cmd + A`) Was Proven

The modifier sentinel proved serialization, platform modifier mapping, selection pipeline, and follow-up overwrite:
1. **Headless (`dioxus-native-dom::test_synthetic_keyboard_events`)**:
   - `input` contained `"a"`.
   - Dispatched `dispatch_synthetic_key(..., Some(input_id), "a", Modifiers::SUPER)` (on macOS) or `CONTROL`.
   - Followed immediately by typing `"z"` without modifiers.
   - Settle/poll confirmed `typed` signal transitioned to `"z"`, proving all previous text was selected and replaced.
2. **Live Native Host (`tests/live_inspect.rs` & `cross_host`)**:
   - `input#test-input` contained `"Continuous reactivity 42"` (in `live_inspect.rs`) or `"H"` (in `cross_host`).
   - Dispatched `select_all(Some(input_id))` / `blitz-host key a --meta --node 4294967413`.
   - Followed by typing `"K"` / `"Z"`.
   - Live semantic inspect confirmed `p#typed-text` transitioned to `"Typed: K"` / `"Typed: Z"`.
   - `cross_host` stdout emitted: `[cross_host] Input value changed: Z`.

---

## 7. Headless Semantics Route

Headless verification was executed via `blitz/packages/dioxus-native-dom/src/events.rs`:
- Implemented `test_synthetic_keyboard_events`.
- Built on `DioxusDocument` + `VirtualDom` + in-memory `BaseDocument`.
- Validated without any OS window, Winit event loop, or display server dependencies in ~0.59s.
- Command:
  ```bash
  cargo test --manifest-path blitz/Cargo.toml -p dioxus-native-dom test_synthetic_keyboard_events
  ```

---

## 8. Live E2E Proof Observed

Live E2E verification was executed against running native desktop hosts with full GPU VSync and Winit windowing:
1. **Automated Integration Test (`tests/live_inspect.rs`)**:
   - Spawns `oxidase-native-runner` in interactive dev mode (Winit 0.31, Vello GPU VSync).
   - Discovers and attaches via UDS socket.
   - Executes click, focus, set-value, settle, visual PNG capture (800x600, 171,392 bytes).
   - Executes keyboard lane: Enter button activation, Space button activation, Shift+Tab focus reversal, Cmd/Ctrl+A select all and overwrite, Backspace character deletion, Escape focus clearing.
   - Command:
     ```bash
     cargo test --test live_inspect --manifest-path util/blitz-host/crates/blitz-host-transport/Cargo.toml -- --nocapture
     ```
     Result: `test test_live_native_runner_attach_and_inspect ... ok (8.54s)`.
2. **Interactive CLI Verification (`cross_host`)**:
   - Ran `cargo run --manifest-path util/oxidase/crates/oxidase/Cargo.toml --example cross_host --features "native,blitz-host"` (PID 79487).
   - `blitz-host list` discovered PID 79487 (`cross_host v0.1.0`).
   - Dispatched `blitz-host focus 4294967417`, `blitz-host key Enter`, `blitz-host key Space` -> observed button incrementing to 1, then 2.
   - Dispatched `blitz-host key Shift+Tab` -> observed focus move to input `#4294967413`.
   - Dispatched `blitz-host key H`, `blitz-host key i` -> observed `p#typed-text` render `"Typed: Hi"`.
   - Dispatched `blitz-host key Backspace` -> observed `p#typed-text` render `"Typed: H"`.
   - Dispatched `blitz-host key a --meta`, `blitz-host key Z` -> observed `p#typed-text` render `"Typed: Z"`.
   - Dispatched `blitz-host key Escape` -> observed focus cleared from input.

---

## 9. What Remains Deferred

The following items are deliberately deferred as out of scope for this bounded core keyboard lane:
1. **Complex IME / Composition Pipelines**: Multi-stage CJK/composition key event sequences (`compositionstart`, `compositionupdate`, `compositionend`) remain deferred to a dedicated IME phase.
2. **Exhaustive Keyboard Shortcuts Suite**: Non-core shortcuts (e.g. multi-cursor editing, word-boundary jumps like `Alt+ArrowLeft`, undo/redo stacks) remain engine-level features of Blitz text inputs rather than host debug primitives.
3. **Platform-Specific Cocoa Selector Synthetics**: Winit native selectors on macOS (`insertNewline:`, `deleteBackward:`) are mapped via standard `Key` and `Code` events; raw Cocoa selector injection is deferred.
4. **Rich Multi-Window Focus Routing**: Routing key actions across multiple concurrently focused windows in a multi-window application is ready via `window_id` in `ActionRequest::Key`, but multi-window interactive tests are deferred.
