# Result: Inspect Output File (`-o`) & DOM-Standard Click Resolution Policy

## 1. Current Repo Facts

1. **`blitz-host` Control Plane Baseline**:
   - `blitz-host` provides an out-of-process control plane and live DOM inspection harness for Blitz and Dioxus Native desktop applications over local Unix Domain Sockets (UDS).
   - Core capabilities prior to this pass included: strict UDS discovery with dead PID reaping, host ambiguity guard (`len == 1` implicit auto-discovery, `len >= 2` requires `--pid`), polymorphic CSS selector targeting (`ElementTarget`), deterministic VSync settlement (`settle(n)`), visual full/cropped capture (`capture -o`), and mouse/keyboard action dispatching.

2. **Previous Inspect Output Limitation**:
   - `blitz-host inspect` previously dumped the entire serialized DOM tree JSON directly to `stdout`.
   - On realistic native applications (100~500+ nodes), output flooded terminals with 700~3,000+ lines of JSON, polluting CI logs and exhausting LLM context windows during automated agent workflows.

3. **Previous Click Resolution Mismatch**:
   - `ActionRequest::Click` previously treated any `click_fn` returning `false` (meaning unhandled by Dioxus VirtualDOM component listeners) as a fatal error (`Err("Node #{node_id} or active listener not found in document")`).
   - In standard W3C DOM and UI automation frameworks (Playwright, Puppeteer), clicking valid DOM elements without explicit click handlers (such as `body` or static backdrop containers) is a valid, essential operation used for outside-click dismiss, focus clearing (blur), or neutral background testing.
   - Conflating "no listener handled the event" with "target is invalid" caused valid selectors like `"body"` to fail with exit code 1 even though the node was accurately resolved in the DOM.

4. **Scope Boundary**:
   - Runtime scripting (Rhai, `eval`, `run`, REPL) remains deferred to the cross-host `oxidase` layer.
   - Same-process multi-window routing and speculative extensions remain deferred.

---

## 2. What I Changed

1. **Protocol Schema & Machine-Readable Interception Field (`crates/blitz-host-protocol/src/lib.rs`)**:
   - Added an optional, backward-compatible `handled` boolean field to `ActionResponse`:
     ```rust
     #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
     #[serde(rename_all = "camelCase")]
     pub struct ActionResponse {
         pub success: bool,
         pub node_id: u64,
         #[serde(default, skip_serializing_if = "Option::is_none")]
         pub handled: Option<bool>,
         #[serde(default, skip_serializing_if = "Option::is_none")]
         pub message: Option<String>,
     }
     ```
   - Exposes a first-class, machine-readable boolean (`handled: Some(true)` vs `handled: Some(false)`) allowing AI agents and CI assertions to programmatically evaluate listener interception without brittle message string parsing.
   - Maintained full deserialization backward-compatibility via `#[serde(default, skip_serializing_if = "Option::is_none")]`.

2. **Decoupled DOM Node Existence from Listener Interception (`crates/blitz-host/src/host.rs`)**:
   - Updated `handle_action` (`ActionRequest::Click`) and `handle_action_click`:
     - **Step 1 (DOM Existence Check)**: Verifies `base_doc.get_node(NodeId::from_u64(node_id))` exists. If absent, fails fast with `Err(format!("Node #{node_id} not found in document"))`.
     - **Step 2 (Listener Execution)**: Calls `let handled = click_fn(base_doc, node_id);`.
     - **Step 3 (Success Dispatch Return)**: Returns `Ok(ActionResponse)` regardless of whether a Dioxus listener was intercepted, with `handled: Some(handled)` and clear diagnostic messages:
       - Handled: `"Dispatched synthetic click to node #{node_id} (handled by listener)"`
       - Unhandled: `"Dispatched synthetic click to node #{node_id} (unhandled, background click)"`
   - Updated `Focus` to set `handled: Some(true)` on successful focus dispatch.
   - Updated `crates/blitz-host-bridge/src/lib.rs` test mock to conform to the updated struct signature.

3. **Optional Output Spill in `inspect` CLI (`crates/blitz-host/src/bin/blitz-host.rs`)**:
   - Added `-o, --output <PATH>` argument parsing to `blitz-host inspect`.
   - **When `-o <PATH>` is supplied**:
     - Serializes pretty JSON and writes it to `<PATH>` (auto-creating parent directories as needed).
     - Prints compact metadata JSON to `stdout`:
       ```json
       {
         "success": true,
         "filePath": "/path/to/dom.json",
         "rootId": 4294967300,
         "nodeCount": 102,
         "bytes": 20359,
         "message": "Saved DOM inspection snapshot (102 nodes) to /path/to/dom.json"
       }
       ```
   - **When `-o` is omitted**:
     - Prints the full `InspectResponse` JSON to `stdout` as before, preserving 100% backward compatibility with shell pipelines and `jq`.
   - **CLI Flag Disambiguation Fix**:
     - Updated `determine_selector` to skip flag value arguments (such as `-o dom.json` or `--selector sel`), preventing output file paths ending in `.json` from being misinterpreted as explicit host descriptor files.
   - Updated `print_inspect_help()` and `README.md` with `-o` documentation and examples.

---

## 3. Validation Actually Run

1. **Protocol Unit Tests & Serde Backward-Compatibility**:
   ```bash
   cargo test -p blitz-host-protocol --lib
   ```
   - **Result**: Passed (2 passed; 0 failed). Verified roundtrip serialization with `handled: true` and deserialization of legacy envelopes omitting `handled`.

2. **Integration Test Suite**:
   ```bash
   cargo test --test live_inspect
   ```
   - **Result**: Passed (14 live E2E assertion stages passed in 10.64s).

3. **Live Native Verification against Interactive Runner (`oxidase-native-runner`, PID 92024)**:

   - **A. `inspect -o <PATH>` (Full Document File Spill)**:
     ```bash
     target/debug/blitz-host inspect -o target/full_dom.json
     ```
     - **Stdout**:
       ```json
       {
         "bytes": 20994,
         "filePath": "/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/target/full_dom.json",
         "message": "Saved DOM inspection snapshot (105 nodes) to /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/target/full_dom.json",
         "nodeCount": 105,
         "rootId": 4294967297,
         "success": true
       }
       ```
     - **File Verified**: `target/full_dom.json` exists on disk with 21KB of formatted DOM JSON.

   - **B. `inspect [TARGET] -o <PATH>` (Subtree File Spill)**:
     ```bash
     target/debug/blitz-host inspect "body" -o target/body_dom.json
     ```
     - **Stdout**:
       ```json
       {
         "bytes": 20359,
         "filePath": "/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/target/body_dom.json",
         "message": "Saved DOM inspection snapshot (102 nodes) to /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/target/body_dom.json",
         "nodeCount": 102,
         "rootId": 4294967300,
         "success": true
       }
       ```
     - **File Verified**: `target/body_dom.json` exists on disk with 20KB of formatted subtree JSON.

   - **C. Standard `inspect` (Without `-o`)**:
     ```bash
     target/debug/blitz-host inspect "#test-input"
     ```
     - **Stdout**: Full `InspectResponse` JSON printed directly to `stdout` with `rootId: 4294967449`.

   - **D. Valid DOM Element Without Listener (`body` - Unhandled Background Click)**:
     ```bash
     target/debug/blitz-host mouse click "body"
     ```
     - **Exit Code**: `0`
     - **Stdout**:
       ```json
       {
         "success": true,
         "nodeId": 4294967300,
         "handled": false,
         "message": "Dispatched synthetic click to node #4294967300 (unhandled, background click)"
       }
       ```

   - **E. Valid DOM Element With Active Listener (`input` - Handled Click)**:
     ```bash
     target/debug/blitz-host mouse click "input"
     ```
     - **Exit Code**: `0`
     - **Stdout**:
       ```json
       {
         "success": true,
         "nodeId": 4294967449,
         "handled": true,
         "message": "Dispatched synthetic click to node #4294967449 (handled by listener)"
       }
       ```

   - **F. Missing Selector / Non-Existent Target (Fail Fast)**:
     ```bash
     target/debug/blitz-host mouse click "#does-not-exist"
     ```
     - **Exit Code**: `1`
     - **Stderr**: `Error: Custom { kind: Other, error: "Element matching selector '#does-not-exist' not found in document" }`

   - **G. Missing Numeric Node ID (Fail Fast)**:
     ```bash
     target/debug/blitz-host mouse click 99999999
     ```
     - **Exit Code**: `1`
     - **Stderr**: `Error: Custom { kind: Other, error: "Node #99999999 not found in document" }`

4. **Documentation Structure Verification**:
   ```bash
   bash ~/.gemini/config/skills/verify-markdown/bin/verify-markdown.sh result.md --require-frontmatter false
   bash ~/.gemini/config/skills/verify-markdown/bin/verify-markdown.sh handoff.md --require-frontmatter false
   ```
   - **Result**: Passed (`[verify-markdown] OK: File structure is valid.`).

---

## 4. Final Verdict

**Implemented and clarified**.

1. `inspect -o <PATH>` works reliably as an optional file spill mechanism, saving large DOM snapshots to disk while emitting clean, structured metadata on `stdout`.
2. Standard `inspect` preserves 100% backward compatibility by emitting full JSON to `stdout` when `-o` is omitted.
3. DOM click resolution correctly decouples node existence from listener interception:
   - Non-existent targets fail fast with exit code 1.
   - Valid targets without active Dioxus listeners (`body`, static backdrops) succeed with exit code 0 and `handled: false`.
   - Valid targets with active Dioxus listeners succeed with exit code 0 and `handled: true`.
4. The handled/unhandled distinction is exposed as a first-class structured field (`handled: Option<bool>`) directly in `ActionResponse` JSON for robust programmatic machine consumption.
5. All live native validations passed with zero regressions.
