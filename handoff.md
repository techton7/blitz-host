# Handoff: Inspect Output Path (`-o`) & DOM-Standard Click Resolution Policy

## 1. Executive Summary & Context

This handoff documents the architectural analysis, design decisions, and execution plan for two related improvements in `blitz-host`:

1. **`inspect -o <PATH>` / `--output <PATH>` Option**:
   - Adding an optional output file flag to `blitz-host inspect` to save the full serialized DOM JSON tree to disk while returning a compact metadata summary on `stdout`.
   - Solves the terminal output flooding and LLM context window bloat caused by dumping 100~500+ node trees directly to `stdout`.
   - Mirrors the ergonomic contract of `capture -o <PATH>`.

2. **DOM-Standard Click Resolution Policy (Background / No-op Click Support)**:
   - Resolving the behavioral mismatch where `mouse click "body"` failed with `Node #4294967300 or active listener not found in document` even though `querySelector("body")` resolved the node accurately to `NodeId(4294967300)`.
   - Decoupling **node existence validation** from **Dioxus event listener interception**:
     - Non-existent target (e.g. `#does-not-exist`) → Returns `Err("Node not found")` (exit code 1).
     - Valid DOM element with active Dioxus listener → Dispatches event and returns `Ok("handled by listener")`.
     - Valid DOM element without active Dioxus listener (e.g. `body`, static containers) → Returns `Ok("unhandled, background click")` conforming to W3C DOM Level 3 Event dispatch semantics.
   - Guarantees 100% forward-compatibility with future Blitz/Dioxus engine evolutions (e.g. when `body` gains native event listeners or arbitrary HTML template mounting).

---

## 2. In-Depth Root Cause & Architectural Analysis

### 2.1. Inspect Output Bloat
* **Current Behavior**:
  `blitz-host inspect [TARGET]` serializes the complete DOM hierarchy and prints the entire formatted JSON string to `stdout`.
* **Impact**:
  In real native Dioxus applications (e.g. `oxidase-native-runner`), documents contain 100~500+ nodes. Serialized output exceeds 700~3,000 lines of JSON per call, overwhelming human operator terminals, polluting test logs, and rapidly exhausting AI agent token context.
* **Solution**:
  Introduce `-o, --output <PATH>`.
  - When `-o <PATH>` is supplied: write the pretty JSON to `<PATH>` and emit a concise 5-line metadata JSON on `stdout`.
  - When `-o` is omitted: preserve 100% backward compatibility by printing the full JSON to `stdout` for pipeability with `jq` and existing scripts.

---

### 2.2. QuerySelector vs. Click Event Dispatch Boundary

#### A. ID Management Scopes: Blitz vs. Dioxus
An essential distinction was identified between the two ID systems operating within `blitz`:

| Dimension | Blitz DOM `NodeId` | Dioxus `data-dioxus-id` (`ElementId`) |
| :--- | :--- | :--- |
| **Scope** | Complete document: `#document`, `<html>`, `<head>`, `<body>`, and all children | Exclusively the Dioxus VirtualDOM component subtree inside `<main id="main">` |
| **Assigned By** | Blitz DOM / Stylo engine during node creation | Dioxus runtime during VirtualDOM component mounting |
| **Purpose** | CSS styling, Taffy layout measurement, visual rendering, `querySelector` | Reactive signal updates, event listener dispatch (`onclick`, `oninput`) |

When `blitz-host inspect "body"` or `blitz-host capture "body"` runs, Blitz's Stylo engine executes `doc.query_selector("body")`. It successfully returns `NodeId(4294967300)`. Both subtree inspection and visual cropping work properly.

#### B. The Root Cause of `mouse click "body"` Failure
When executing `blitz-host mouse click "body"`:
1. `querySelector("body")` successfully resolves the target to `NodeId(4294967300)`.
2. `blitz-host` calls `dioxus_native::dispatch_synthetic_click(doc, node_id, modifiers)`.
3. In `dioxus-native-dom/src/events.rs:680-696`:
   ```rust
   let mut current = Some(node_id);
   let mut target = None;
   while let Some(id) = current {
       if let Some(node) = doc.get_node(id) {
           if let Some(dioxus_id) = crate::dioxus_document::get_dioxus_id(node) {
               target = Some((dioxus_id, id));
               break;
           }
           current = node.parent;
       } else {
           break;
       }
   }
   let Some((dioxus_id, target_node_id)) = target else {
       return false; // <-- Returns false because body, html, document have no Dioxus ID!
   };
   ```
4. In `crates/blitz-host/src/host.rs:194-202`:
   ```rust
   if click_fn(base_doc, node_id) {
       Ok(ActionResponse { success: true, .. })
   } else {
       Err(format!("Node #{node_id} or active listener not found in document")) // <-- Fatal error!
   }
   ```
5. `blitz-host` conflated `click_fn returning false` (unhandled by Dioxus) with a fatal protocol error, failing with exit code 1.

#### C. Web Standards vs. Strict Synthetic Event Assumptions
In W3C DOM Level 3 Events and automated browser drivers (Playwright, Puppeteer, Selenium):
- Clicking `body` or a static background `div` is a standard, essential user action (used for outside-click dismiss of popovers/menus, clearing active input focus via blur, or testing neutral canvas clicks).
- When a click event is dispatched to an element without an `onclick` listener, the event bubbles to `window` and completes with no-op. It **never throws an error or fails the test command**.
- `blitz-host` should therefore distinguish between:
  1. **Invalid Target Error**: Target does not exist in the document (`base_doc.get_node(node_id).is_none()`).
  2. **Valid Target Success**: Target exists in DOM. Click was dispatched. Whether a Dioxus listener was intercepted (`handled == true`) or not (`handled == false`) is purely informative status metadata, not a failure.

---

## 3. Forward-Compatibility Guarantee

The proposed fix in `blitz-host` does not use hardcoded tag bypasses (`if node == "body"`). Instead, it follows standard event dispatch lifecycle semantics:

```rust
// 1. Verify target node exists in DOM
let node = base_doc
    .get_node(blitz_dom::NodeId::from_u64(node_id))
    .ok_or_else(|| format!("Node #{node_id} not found in document"))?;

// 2. Dispatch event to Dioxus listeners (bubbles up to VirtualDOM root)
let handled = click_fn(base_doc, node_id);

// 3. Return success with informative dispatch status
Ok(ActionResponse {
    success: true,
    node_id,
    message: Some(if handled {
        format!("Dispatched synthetic click to node #{node_id} (handled by listener)")
    } else {
        format!("Dispatched synthetic click to node #{node_id} (unhandled, no active listener)")
    }),
})
```

### Why this is 100% forward-compatible:
1. **Present State (No `body` listener in Dioxus)**:
   - `node_id` exists in `base_doc`.
   - `click_fn` returns `false`.
   - `blitz-host` succeeds with `exit code 0` and reports `"(unhandled, no active listener)"`.
   - Outside click / background click automation succeeds without breaking.
2. **Future State (Blitz/Dioxus supports `body.onclick` or arbitrary template roots)**:
   - When Blitz allows mounting Dioxus directly to `body` or attaching listeners to `body`, `get_dioxus_id` will locate the listener on `body`.
   - `click_fn` will return `true` and invoke the user callback.
   - `blitz-host` immediately observes `handled == true` and reports `"(handled by listener)"`.
   - Zero changes to `blitz-host` will be required when upstream engines evolve.

---

## 4. Technical Specifications & API Contract

### 4.1. `inspect` Command Line Interface

```text
USAGE:
    blitz-host inspect [TARGET] [OPTIONS] [DESCRIPTOR_PATH]

ARGUMENTS:
    [TARGET]                 Optional root element specified by numeric node ID or CSS selector
                             to inspect only a specific subtree (e.g. 4294967464, "#test-input", "body").
    [DESCRIPTOR_PATH]        Path to host descriptor JSON file or UDS socket (auto-discovered if omitted).

OPTIONS:
    -o, --output <PATH>      Save full formatted DOM JSON to file and print compact metadata on stdout
    -n, --node <NODE_ID>     Target specific root node/subtree ID
    -s, --selector <SEL>     Target specific root element by CSS selector
        --pid <PID>          Target specific host process by OS process ID
        --window <ID>        Target specific window ID (defaults to primary window)
    -h, --help               Print help information
```

#### Output Specifications:
* **Without `-o`**:
  Prints full `InspectResponse` JSON to `stdout` (existing behavior, 100% backward compatible).
* **With `-o <PATH>`**:
  Writes full `InspectResponse` JSON to `<PATH>`.
  Outputs compact metadata JSON on `stdout`:
  ```json
  {
    "success": true,
    "filePath": "/absolute/path/to/dom.json",
    "rootId": 4294967300,
    "nodeCount": 102,
    "bytes": 28412,
    "message": "Saved DOM inspection snapshot (102 nodes) to /absolute/path/to/dom.json"
  }
  ```

---

### 4.2. Action Click Resolution Specification

* **Protocol & Response (`crates/blitz-host-protocol`)**:
  Add an optional, backward-compatible structured `handled` boolean field to `ActionResponse`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ActionResponse {
      /// Whether the action was successfully dispatched to a valid target.
      pub success: bool,
      /// Target node ID that received the action.
      pub node_id: u64,
      /// Whether an active event listener intercepted and handled this action.
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub handled: Option<bool>,
      /// Optional human-readable status or diagnostic message.
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub message: Option<String>,
  }
  ```

* **Status Matrix & Machine-Readable Contract**:

| Target Condition | DOM Check | `click_fn` Return | `success` | `handled` | Exit Code | Message |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| Selector not found | Fails | N/A | `false` | `None` | 1 | Stderr: `No element matching selector '...'` |
| Invalid Node ID | Fails | N/A | `false` | `None` | 1 | Stderr: `Node #... not found in document` |
| Element with active listener | Exists | `true` | `true` | `Some(true)` | 0 | `"Dispatched synthetic click to node #... (handled by listener)"` |
| Element without listener (`body`, static container) | Exists | `false` | `true` | `Some(false)` | 0 | `"Dispatched synthetic click to node #... (unhandled, no active listener)"` |

> [!NOTE]
> By surfacing `handled: bool` directly in the JSON response, AI agents and automated CI test runners can programmatically assert `response.handled == Some(true)` or `response.handled == Some(false)` without brittle message string parsing.

---

## 5. Implementation Plan

### Step 1: Update Protocol & `host.rs` Action Click Handlers
- In `crates/blitz-host-protocol/src/lib.rs`:
  - Add `#[serde(default, skip_serializing_if = "Option::is_none")] pub handled: Option<bool>` to `ActionResponse`.
- In `crates/blitz-host/src/host.rs`:
  - Update `handle_action`: In `ActionRequest::Click`, check `base_doc.get_node(NodeId::from_u64(node_id))` first. Return `Err` if not found. If found, call `click_fn` and return `Ok(ActionResponse { success: true, node_id, handled: Some(handled), message: ... })`.
  - Update `handle_action_click`: Apply the identical existence check and decoupled return policy with `handled: Some(handled)`.

### Step 2: Implement `-o / --output` in `blitz-host inspect`
- In `crates/blitz-host/src/bin/blitz-host.rs`:
  - In `subargs` parsing for `inspect`, parse `-o` / `--output <PATH>`.
  - Exclude `-o` and `--output` values from `positional` selector parsing.
  - If output path is present:
    - Format response JSON (`serde_json::to_string_pretty`).
    - Write to file (`std::fs::write(&target_path, json_str)`).
    - Emit compact metadata JSON on `stdout`.
  - If output path is absent:
    - Emit full JSON on `stdout` as before.
  - Update `print_inspect_help()` to document `-o, --output <PATH>`.

### Step 3: Verification & Live Native Proof
- Execute unit and integration tests:
  - `cargo check --workspace`
  - `cargo test -p blitz-host-transport`
- Live native verification against running `oxidase-native-runner`:
  1. `blitz-host inspect -o target/full_dom.json` → Confirm file created and compact stdout emitted.
  2. `blitz-host inspect "body" -o target/body_dom.json` → Confirm subtree saved.
  3. `blitz-host mouse click "body"` → Confirm exit code 0 and `nodeId: 4294967300`.
  4. `blitz-host mouse click "input"` → Confirm exit code 0 and listener handled.
  5. `blitz-host mouse click "#does-not-exist"` → Confirm exit code 1 with clean error.
