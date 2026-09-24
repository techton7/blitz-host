# Result: Implementation, Fix, and Verification of Blitz Host Core Mouse / Pointer / Wheel Lane

## 1. Executive Summary

This pass resolved the compile/fixture breaks and completed the core mouse, pointer, and wheel interaction lane for `blitz-host` and `dioxus-native-dom` per `instruction.md`. The entire workspace test suite now passes cleanly from `Cargo.toml` down to live native E2E integration tests.

1. **Root Cause Analysis & Fixes**:
   - **Transport Unit Test Fixture Break**: In `crates/blitz-host-transport/src/lib.rs`, `test_transport_roundtrip_server_client` failed compilation (`E0063`) because mock initializers for `SemanticNode` and `InspectResponse` lacked the newly added fields (`active`, `hovered`, `scroll_offset`, `hover_node_id`, `viewport_scroll`). Fixed by populating all newly added fields with appropriate test defaults.
   - **Inspect Bridge Text Node Panic**: In `crates/blitz-host-bridge/src/inspect.rs`, calling `node.scroll_offset()` unconditionally panicked on `#text` and `#comment` nodes (`layout_data is not available on this node kind`), causing `"Host UI thread bridge disconnected"`. Fixed by pattern matching `NodeData::Element(_) | NodeData::AnonymousBlock(_) | NodeData::Document(_)` before reading scroll offsets.
   - **Public Re-export Parity**: In `blitz/packages/dioxus-native-dom`, `MouseEventButton` was made a public re-export (`pub use blitz_traits::events::MouseEventButton`), allowing `dioxus-native` and `blitz-host` to consume typed mouse buttons without reaching into internal private modules.
2. **Comprehensive Green Baseline Verified**:
   - `cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture` passes 100% across all crates (`blitz-host`, `blitz-host-bridge`, `blitz-host-protocol`, `blitz-host-transport`, and `live_inspect`).
   - `cargo test --manifest-path blitz/Cargo.toml -p dioxus-native-dom -- test_synthetic` passes 100% across all focus/input, pointer, and keyboard synthetic tests.
   - Live CLI commands (`move`, `down`, `up`, `wheel`, `drag`, `capture`) verified against running `cross_host --features blitz-host`.

---

## 2. What Was Broken and What Was Fixed

### 2.1 Transport Test Fixtures
- **Broken**:
  `crates/blitz-host-transport/src/lib.rs` failed during `cargo test`:
  ```text
  error[E0063]: missing fields `active`, `hovered` and `scroll_offset` in initializer of `SemanticNode`
  error[E0063]: missing fields `hover_node_id` and `viewport_scroll` in initializer of `InspectResponse`
  ```
- **Fixed**:
  Updated `InspectResponse` and `SemanticNode` fixture in `test_transport_roundtrip_server_client` to explicitly supply `hover_node_id: None`, `viewport_scroll: None`, `hovered: None`, `active: None`, and `scroll_offset: None`.

### 2.2 Bridge Inspection for Non-Layout Nodes
- **Broken**:
  `node.scroll_offset()` delegates to `self.layout_data().scroll_offset`. In `blitz-dom`, `layout_data()` panics for node kinds that do not participate in layout (such as `#text` and `#comment` nodes). This resulted in an engine panic and broken pipe during live inspect traversal.
- **Fixed**:
  Updated `crates/blitz-host-bridge/src/inspect.rs`:
  ```rust
  let scroll_offset = match &node.data {
      NodeData::Element(_) | NodeData::AnonymousBlock(_) | NodeData::Document(_) => {
          let offset = *node.scroll_offset();
          if offset.x != 0.0 || offset.y != 0.0 {
              Some([offset.x, offset.y])
          } else {
              None
          }
      }
      _ => None,
  };
  ```

### 2.3 MouseEventButton Re-export
- **Broken**:
  `MouseEventButton` was imported privately in `events.rs`, making `blitz-host` fail to resolve `dioxus_native::MouseEventButton`.
- **Fixed**:
  Re-exported `pub use blitz_traits::events::MouseEventButton;` from `dioxus-native-dom::events` and `dioxus-native-dom::lib`, aligning with `dioxus-native` exports.

### 2.4 Hover Target Identity Mismatch and Final Hover Contract
- **What the hover mismatch actually was**:
  In `crates/blitz-host-transport/tests/live_inspect.rs`, the live test dispatched `client.hover(card_id)` on `#mouse-test-card`. The test originally asserted that `InspectResponse.hover_node_id` MUST be strictly equal to `Some(card_id)`. However, `#mouse-test-card` has a child `span` (`"Pointer Target (Move / Down / Up / Drag)"`). In the Blitz DOM engine (`BaseDocument::set_hover_to(x, y)`), hit-testing walks to the deepest non-anonymous layout leaf under the cursor coordinates (the inner `span`, node `#4294967465`) and sets `self.hover_node_id` to that leaf node. The test panicked because `Some(4294967465) != Some(4294967464)`.
- **Whether the implementation or the proof expectation changed**:
  Both the contract and the proof model were aligned to reflect DOM reality without synthetic shortcuts:
  1. *Bridge / Inspect contract*: Kept truthful to the engine. `InspectResponse.hover_node_id` publishes the deepest directly hit layout node from Blitz (`doc.get_hover_node_id()`). Simultaneously, `SemanticNode.hovered` publishes `node.is_hovered()`. Because Blitz's `set_hover_to` walks the entire ancestor chain from the leaf up to root and marks each ancestor as `ElementState::HOVER`, the parent card container (`#mouse-test-card`) AND its inner children report `hovered: Some(true)`.
  2. *Proof expectation*: `live_inspect.rs` was updated to assert:
     - `InspectResponse.hover_node_id` matches the card itself OR one of its descendant children (`card_id == hid || post_hover_card.children.contains(&hid)`).
     - The target card's `SemanticNode.hovered` reports `Some(true)`.
     - The Dioxus `onmouseenter` handler on `#mouse-test-card` executed, proving that reactive state changed (`#hover-status` -> `"HOVERED"`).
- **The Final Hover Contract**:
  - `InspectResponse.hover_node_id: Option<u64>`: The raw innermost hit layout node under the pointer coordinates.
  - `SemanticNode.hovered: Option<bool>`: Truthful per-node hover state (`node.is_hovered()`), propagated along the ancestor chain by the engine.
  - `client.hover(node_id)` / `client.mouse_move(..., Some(node_id), ...)`: Targets the spatial center bounding rect of `node_id` and dispatches coordinates to Blitz.
  - Event propagation: Non-bubbling `mouseenter`/`pointerenter` events reach container listeners when mouse enters descendant elements.

---

## 3. Pointer / Wheel Action Surface Added

### 3.1 Protocol Vocabulary (`crates/blitz-host-protocol`)

```rust
pub enum ActionRequest {
    ...
    MouseMove {
        window_id: Option<u64>,
        node_id: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
        modifiers: Option<KeyModifiers>,
    },
    MouseDown {
        window_id: Option<u64>,
        node_id: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
        button: Option<String>,
        modifiers: Option<KeyModifiers>,
    },
    MouseUp {
        window_id: Option<u64>,
        node_id: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
        button: Option<String>,
        modifiers: Option<KeyModifiers>,
    },
    Wheel {
        window_id: Option<u64>,
        node_id: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
        delta_x: f64,
        delta_y: f64,
        modifiers: Option<KeyModifiers>,
    },
}

pub struct SemanticNode {
    ...
    pub hovered: Option<bool>,
    pub active: Option<bool>,
    pub scroll_offset: Option<[f64; 2]>,
}

pub struct InspectResponse {
    ...
    pub hover_node_id: Option<u64>,
    pub viewport_scroll: Option<[f64; 2]>,
}
```

### 3.2 Client Transport Surface (`crates/blitz-host-transport`)

```rust
// Movement & hover
client.mouse_move(window_id, node_id, coords, modifiers)?;
client.hover(node_id)?;
client.move_to(node_id)?;
client.move_to_coords(x, y)?;

// Press & release
client.mouse_down(window_id, node_id, coords, button, modifiers)?;
client.mouse_up(window_id, node_id, coords, button, modifiers)?;

// Composed drag
client.drag(from_node_id, to_node_id)?;

// Wheel & scroll
client.wheel(window_id, node_id, coords, delta_x, delta_y, modifiers)?;
client.scroll(node_id, delta_y)?;
```

### 3.3 CLI Command Surface (`crates/blitz-host`)

```bash
blitz-host move <NODE_ID> [--x X] [--y Y] [--pid PID]
blitz-host down <NODE_ID> [--button BUTTON] [--pid PID]
blitz-host up <NODE_ID> [--button BUTTON] [--pid PID]
blitz-host wheel <NODE_ID> --dy <DY> [--dx <DX>] [--pid PID]
blitz-host drag <FROM_NODE_ID> <TO_NODE_ID> [--pid PID]
```

---

## 4. How Each Interaction Primitive Was Proven

### 4.1 Hover / Move Proof
1. **Headless (`test_synthetic_pointer_events`)**:
   - Injected synthetic pointer move to target node.
   - Verified `doc.is_node_hovered(target_id) == true` and Dioxus `onmouseenter` handler fired.
2. **Live Native Host (`live_inspect.rs` & `cross_host`)**:
   - Dispatched `client.hover(card_id)` on `#mouse-test-card`.
   - Verified `hover_node_id` matches hit target, card reports `hovered == Some(true)`, and `#hover-status` renders `"HOVERED"`.

### 4.2 Down / Up / Drag Proof
1. **Headless (`test_synthetic_pointer_events`)**:
   - Injected synthetic pointer down -> verified `doc.is_node_active(target_id) == true` and Dioxus `onpointerdown` fired.
   - Injected synthetic pointer up -> verified `doc.is_node_active(target_id) == false` and Dioxus `onpointerup` fired.
2. **Live Native Host (`live_inspect.rs` & `cross_host`)**:
   - Dispatched `mouse_down` -> inspect verified active state and `#pressed-status` rendered `"PRESSED"`.
   - Dispatched `mouse_up` -> inspect verified active state cleared and `#pressed-status` rendered `"RELEASED"`.
   - Dispatched `drag(card_id, button_id)` -> composed `move` -> `down` -> `move` -> `up` with VSync settling.

### 4.3 Wheel / Scroll Proof
1. **Headless (`test_synthetic_pointer_events`)**:
   - Injected synthetic wheel delta (0.0, 42.0) -> verified Dioxus `onwheel` observed `delta_y == 42.0`.
2. **Live Native Host (`live_inspect.rs` & `cross_host`)**:
   - Dispatched `scroll(scroll_id, 45.0)` / `blitz-host wheel 4294967466 --dy 50`.
   - Inspect verified `#scroll-status` updated to `"Scroll Y: 45"` / `"Scroll Y: 50"`.

---

## 5. Scope Adjustments

Per `instruction.md` Section 3 and the approved implementation plan:
- **No CLI alias proliferation**: Dedicated CLI aliases (`hover`, `scroll`) were avoided in the first pass to keep the CLI command surface clean and minimal (`move`, `down`, `up`, `wheel`).
- **Drag composition**: `drag` is cleanly composed from existing primitives (`move` -> `down` -> `move` -> `up`) rather than inventing a separate low-level kernel event.

---

## 6. Validation Actually Run and Current Status

1. **Full `blitz-host` Workspace Test Suite**:
   ```bash
   cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture
   ```
   **Output**:
   - `blitz-host`: 0 unit tests passed.
   - `blitz-host-bridge`: 3 tests passed (`test_bridge_focus_and_set_value_actions`, `test_inspect_document_minimal`, `test_bridge_capture_document`).
   - `blitz-host-protocol`: 2 tests passed (`test_descriptor_serde_roundtrip`, `test_control_envelope_serde_roundtrip`).
   - `blitz-host-transport`: 1 unit test passed (`test_transport_roundtrip_server_client`).
   - `live_inspect` integration test: 1 passed (all 11 live steps: attach, click, focus, set-value, keyboard, hover, down, up, drag, wheel, visual capture).
   - All doc tests passed.
   - **Result: `ok. 7 passed; 0 failed` across all suites.**

2. **Headless Semantics Suite**:
   ```bash
   cargo test --manifest-path blitz/Cargo.toml -p dioxus-native-dom -- test_synthetic
   ```
   **Output**:
   - `test_synthetic_focus_and_input_events ... ok`
   - `test_synthetic_pointer_events ... ok`
   - `test_synthetic_keyboard_events ... ok`
   - **Result: `ok. 3 passed; 0 failed`.**

3. **Live Interactive CLI against Canonical `cross_host`**:
   - Launched `cargo run -p oxidase --example cross_host --features blitz-host` (PID 30402).
   - Ran `blitz-host list` -> discovered PID 30402.
   - Ran `blitz-host move 4294967457` -> inspect showed `#hover-status` `"HOVERED"`.
   - Ran `blitz-host down 4294967457` -> inspect showed `#pressed-status` `"PRESSED"`.
   - Ran `blitz-host up 4294967457` -> inspect showed `#pressed-status` `"RELEASED"`.
   - Ran `blitz-host wheel 4294967466 --dy 50` -> inspect showed `#scroll-status` `"Scroll Y: 50"`.
   - Ran `blitz-host drag 4294967457 4294967446` -> successfully completed drag sequence.
   - Ran `blitz-host capture` -> generated `blitz-capture-30402.png` (800x600, 132,361 bytes).

---

## 7. What Remains Deferred

1. **Full Gesture Language**: Multi-touch pinch-to-zoom, rotation, and multi-finger pan gestures remain deferred.
2. **Touch / Multi-Touch Event Complexities**: Touch identifiers and multi-touch tracking remain deferred.
3. **Advanced Pointer Capture Semantics**: Explicit W3C `setPointerCapture` / `releasePointerCapture` APIs are deferred.
4. **Isolated Subtree Scene Specialization**: Isolated node-only rendering without scene context (current implementation uses crop-based capture to preserve real visual styling and background context).
5. **Perceptual Visual Diff Engine**: Automatic image comparison (`image-compare`/`dssim`) remains a client/harness responsibility rather than host bridge code.

---

## 8. Subtree / Node-Level Visual Capture (Crop-Based Capture)

### 8.1 What Request Surface Changed
1. **`CaptureRequest` (`crates/blitz-host-protocol`)**:
   - Added `pub node_id: Option<u64>` (with `#[serde(default, skip_serializing_if = "Option::is_none")]`).
   - When `node_id` is `None`, executes full-window capture (`800x600`).
   - When `node_id` is `Some(id)`, executes crop-based capture to the target node's visual bounds.
2. **`CaptureResponse` (`crates/blitz-host-protocol`)**:
   - Added `pub node_id: Option<u64>` publishing the target node ID that was cropped.
3. **`DebugClient` (`crates/blitz-host-transport`)**:
   - Added `client.capture_node(node_id: u64) -> io::Result<CaptureResponse>`.
   - Added `client.capture_window_node(window_id, node_id) -> io::Result<CaptureResponse>`.
   - Added `client.capture_node_to_file(node_id, path) -> io::Result<(u32, u32, PathBuf)>`.
4. **CLI Surface (`crates/blitz-host`)**:
   - Added `blitz-host capture [NODE_ID] [--node <ID>] [-o OUTPUT] [--pid PID] [--json]`.
   - Default filename for node captures: `blitz-capture-<PID>-node-<NODE_ID>.png`.

### 8.2 Crop-Based vs. True Subtree Render
Per `instruction.md` Section 6, this is explicitly a **crop-based capture (`full-scene render followed by crop`)**:
- The document's live scene graph is rasterized into an RGBA pixel buffer using `blitz_paint::paint_scene` and `anyrender_vello_cpu::VelloCpuImageRenderer`.
- When `node_id` is specified, the node's document-relative bounding box is extracted and scaled by the viewport display scale factor.
- A 2D rectangular slice of RGBA pixels corresponding to the node is extracted row-by-row and compressed into PNG bytes using `png::Encoder`.
- **Why this is preferred**: It preserves true on-screen visual reality (actual parent background color, CSS inheritance, text anti-aliasing, and drop shadows) without synthetic rendering isolation or heavy external dependencies.

### 8.3 Node Bounds Resolution and Coordinate Mapping
- Target node lookup: `doc.get_node(NodeId::from_u64(node_id))`.
- Document-relative position: `node.absolute_position(0.0, 0.0)` recursively accumulates parent layout locations and subtracts layout scroll offsets.
- Element dimensions: `node.final_layout().size` provides computed width and height in CSS points.
- Physical pixel scaling: `(pos.x * scale, pos.y * scale, size.width * scale, size.height * scale)` maps directly to the RGBA buffer coordinates.
- Bounds clamping: Coordinates are clamped to the physical viewport bounds `[0..width, 0..height]`.
- Honest error reporting: If a node has non-positive dimensions or lies completely outside the viewport, an honest error `Err("Target node #... has no visible pixels in viewport...")` is returned rather than returning an empty or fake artifact.

### 8.4 Live Native Proof & Visual Artifact Verification
1. **Full-Window vs. Node Crop Verification (`crates/blitz-host-transport/tests/live_inspect.rs`)**:
   - Full-window capture produced `800x600` PNG (146,723 bytes).
   - Node capture on `#mouse-test-card` (node `#4294967464`) produced `618x40` PNG (7,443 bytes).
   - Assertions proved:
     - `node_cap_resp.success == true`
     - `node_cap_resp.node_id == Some(4294967464)`
     - `node_cap_resp.width < cap_resp.width` (618 < 800)
     - `node_cap_resp.height < cap_resp.height` (40 < 600)
     - File on disk: `target/live_proof_node_card.png` verified via `file`: `PNG image data, 618 x 40, 8-bit/color RGBA, non-interlaced`.
2. **Distinct Proof Target**:
   - Node capture on `#test-interaction-button` verified clean crop.
3. **Invalid Node Target**:
   - Node capture on `#9999999` reported `success == false` with message `"Target node #9999999 not found in document"`.
4. **Non-Regressing Baseline**:
   - Full workspace test suite passes `ok. 7 passed; 0 failed`.
   - Headless semantics suite passes `ok. 3 passed; 0 failed`.

---

## 9. CLI Ergonomization, Always-On JSON, Mandatory `-o`, and Metadata JSON

### 9.1 What Public Surface Changed
1. **Always-On JSON Protocol**:
   - The `--json` flag was removed from all CLI subcommands (`list`, `inspect`, `capture`, `click`, `focus`, `set-value`, `key`, `mouse *`).
   - `stdout` is now reserved strictly for valid, pretty-printed JSON payloads across all subcommands.
   - All diagnostic, connection, and VSync settle progress logs were redirected to `stderr` (`eprintln!`), guaranteeing that `stdout` is pipeable to `jq` or AI agent tool parsers without corruption.
2. **Mandatory Output Path for `capture` (`-o` / `--output`)**:
   - `-o <PATH>` / `--output <PATH>` is now strictly required for all `capture` invocations (both full-window and node crop).
   - Omitting `-o` immediately fails with exit code `1` and a clear error message.
   - Ambiguous default fallback filenames (e.g. `blitz-capture-<PID>.png`) and scattered disk artifacts are completely eliminated.
3. **Removal of Inline Base64 from Public JSON Response**:
   - The public CLI response no longer dumps raw `data_base64` image strings to stdout.
   - `capture` writes the image bytes directly to the mandatory `-o` file path on disk, and emits lightweight `CaptureMetadataResponse` JSON.
4. **Compound Key Specification (`key`)**:
   - Removed separate modifier flags (`--shift`, `--ctrl`, `--alt`, `--meta`, `--cmd`).
   - Added compound key parser supporting `+` delimiter (e.g. `cmd+a`, `command+shift+z`, `shift+tab`, `enter`, `escape`, `ctrl+c`).
   - Modifiers (`cmd`, `command`, `meta`, `super`, `shift`, `ctrl`, `control`, `alt`, `opt`, `option`) and named keys (`tab`, `enter`, `return`, `esc`, `escape`, `backspace`, `del`, `delete`, `space`, `arrow*`) are matched case-insensitively.
5. **`mouse` Namespace Hierarchy**:
   - Grouped pointer interactions under `blitz-host mouse <move|down|up|wheel|drag>`, cleaning up top-level CLI namespace pollution.
   - Maintained top-level `move`, `down`, `up`, `wheel`, `drag` as backward-compatible aliases.
6. **Hierarchical `--help` / `-h` Documentation**:
   - Added consolidated `blitz-host mouse --help` namespace documentation.
   - Updated main help and individual subcommand help manuals to reflect compound keys and default JSON format.

### 9.2 Public Metadata JSON Format
The `blitz-host capture` command now outputs strictly typed metadata JSON on `stdout`:
```json
{
  "success": true,
  "filePath": "target/cli_proof_node_card.png",
  "width": 618,
  "height": 40,
  "format": "png",
  "nodeId": 4294967464,
  "bytes": 7443,
  "message": "Captured node #4294967464 cropped to 618x40 PNG visual screenshot (7443 bytes)"
}
```

### 9.3 Live Validation Evidence
1. **Live Native Host E2E Suite (`crates/blitz-host-transport/tests/live_inspect.rs` Step 13)**:
   - `blitz-host list` verified: produces valid JSON array containing live host PID.
   - `blitz-host inspect` verified: produces valid JSON object with `documentId: 1`.
   - `blitz-host key cmd+a` verified: dispatches compound shortcut with `success: true`.
   - `blitz-host mouse move` verified: dispatches pointer movement via namespace with `success: true`.
   - `blitz-host capture` without `-o` verified: exits with code `1` and explains `-o` requirement.
   - `blitz-host capture 4294967464 -o target/cli_proof_node_card.png` verified: writes valid PNG to disk and outputs metadata JSON with no `dataBase64` property.
2. **Automated Suite Results**:
   - `cargo test --manifest-path util/blitz-host/Cargo.toml -- --nocapture`: `ok. 7 passed; 0 failed`.
   - `cargo test --manifest-path blitz/Cargo.toml -p dioxus-native-dom -- test_synthetic`: `ok. 3 passed; 0 failed`.

### 9.4 What Remains Deferred
1. **Multi-touch Gestures**: Pinch-to-zoom and multi-finger pan remain deferred.
2. **Interactive REPL Session**: Persistent `blitz-host shell` keep-alive mode remains a future priority.
