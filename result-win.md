# Result (Windows): `blitz-host` Windows Gaps Hardening and Validation

## 1. Current Repo Facts

1. **Host Environment**:
   - Operating System: Windows 11 Pro (x86_64-pc-windows-msvc)
   - Toolchain: Rust 1.98.1 (MSVC), Python 3.12.10 configured in `PATH` and `PYTHON3`
   - Workspaces:
     - `d:\business\dioxus\util\blitz-host` (`crates/blitz-host-protocol`, `crates/blitz-host-transport`, `crates/blitz-host-bridge`, `crates/blitz-host`)
     - `d:\business\dioxus\util\oxidase` (`crates/oxidase`, `crates/oxidase-macro`, `crates/oxidase-native-runner`, examples)
   - IPC Transport: Windows Named Pipes (`\\.\pipe\blitz-host-{instance_id}`) using `interprocess` 2.4.4.

2. **Starting Gaps Identified in `instruction-win.md`**:
   - **Gap A (`tests/live_inspect.rs`)**:
     - Previously hardcoded the macOS host path (`/Volumes/HDD-1T-2021-Mac/.../oxidase-native-runner`), offering no honest path resolution on Windows and immediately bailing or failing.
   - **Gap B (Explicit `\\.\pipe\...` Targeting)**:
     - CLI selector logic in `blitz-host.rs` only classified explicit endpoints by checking `.json` or `.sock` extensions; explicit Named Pipe paths (`\\.\pipe\...`) were either misinterpreted as positional element targets / CSS selectors or fell back to auto-discovery.
     - `discovery.rs` executed `if !path.exists()` before inspecting pipe paths. In Windows, `Path::new(r"\\.\pipe\...").exists()` is always `false` because Named Pipes are device namespaces, causing any explicit Named Pipe selector to return `io::ErrorKind::NotFound` without attempting connection.

---

## 2. What I Changed

### Gap A: Dynamic Native Runner Resolution (`crates/blitz-host-transport/tests/live_inspect.rs`)
- Replaced the hardcoded macOS absolute path (`/Volumes/.../oxidase-native-runner`) with dynamic resolution function `resolve_native_runner_bin()`:
  1. **Environment Override**: Checks `OXIDASE_RUNNER_BIN` for explicit caller/CI overrides.
  2. **Manifest-Relative Workspace Resolution**: Probes relative locations:
     - `../../../oxidase/crates/oxidase-native-runner/target/debug/oxidase-native-runner`
     - `../../../oxidase/target/debug/oxidase-native-runner`
     - `../../target/debug/oxidase-native-runner`
  3. **Windows Binary Suffix**: On Windows (`#[cfg(windows)]`), appends `.exe` and verifies path existence.
  4. **Platform CLI Binary**: Updates `cli_path` resolution to check `.exe` on Windows instead of assuming a bare Unix binary.
  5. **Child Pipe Draining**: Spawns background drain threads for child `stdout` and `stderr` to prevent OS pipe buffer deadlocks on Windows anonymous pipes.
  6. **Mount Readiness Synchronization**: Extended connection polling loop to wait until `primary_document_id.is_some()`, guaranteeing that `BlitzHost::onmounted` and Document initialization have completed before dispatching commands.

### Gap B: Explicit Windows Named Pipe Targeting (`discovery.rs` & `blitz-host.rs`)
- **`crates/blitz-host-transport/src/discovery.rs`**:
  - In `discover_target`, placed `path_str.starts_with(r"\\.\pipe\")` **ahead** of the filesystem `!path.exists()` check.
  - Constructs a synthetic `HostDescriptor` targeting the explicit pipe and performs a live reachability probe via `is_reachable(&desc)`:
    - If the pipe is active and accepting connections, returns `Ok(desc)`.
    - If unreachable/closed, returns `io::Error::new(io::ErrorKind::ConnectionRefused, ...)`.
- **`crates/blitz-host/src/bin/blitz-host.rs`**:
  - Added helper `is_explicit_descriptor_or_socket(arg: &str)` checking `.json`, `.sock`, and `\\.\pipe\`.
  - Updated `determine_selector()` to return `TargetSelector::ExplicitPath(PathBuf::from(arg))` when `\\.\pipe\` is detected.
  - Updated positional argument parsing across all subcommands (`inspect`, `capture`, `focus`, `set-value`, `key`, and mouse `click`, `move`, `down`, `up`, `wheel`, `drag`) to exclude `\\.\pipe\...` paths from being interpreted as positional `ElementTarget`s (CSS selectors or node IDs).

### Dedicated Proof in `crates/blitz-host-transport/tests/live_cli_attach.rs`
- Expanded the integration test to prove explicit Named Pipe targeting end-to-end:
  1. **Explicit Pipe CLI Inspect**: Invokes `blitz-host inspect \\.\pipe\blitz-host-{instance_id}` directly and asserts valid DOM tree JSON output.
  2. **Explicit Pipe Programmatic Connect**: Exercises `DebugClient::connect_target(&TargetSelector::ExplicitPath(PathBuf::from(&socket_path)))` and runs typed `inspect()`.
  3. **Dead Pipe Programmatic Rejection**: Connects to `\\.\pipe\blitz-host-nonexistent-endpoint-test` and asserts it fails with `io::ErrorKind::ConnectionRefused`.
  4. **Dead Pipe CLI Rejection**: Invokes CLI against the nonexistent pipe and asserts exit code 1 with clear diagnostic error on stderr.
  5. **Clean Connection Teardown**: Ensures `explicit_client` is dropped before server shutdown so that connection reader threads terminate cleanly and server wakers join without blocking.

### Post-Review Findings & Proof Harness Hardening
During supervisor review between the synced copies, two important review-time issues were identified and addressed:
1. **Unix Compile Regression in `crates/blitz-host-transport/src/server.rs`**:
   - The initial Windows cross-platform socket adapter introduced a borrowed socket-name lifetime issue on Unix targets (`as_path().to_fs_name::<GenericFilePath>()?`).
   - Fixed by calling `.into_owned()` on the socket name, restoring 100% clean compilation on Unix/macOS without impacting Windows Named Pipe functionality.
2. **Elimination of Stale CLI Artifacts in Test Harnesses**:
   - The integration tests (`live_cli_attach.rs` and `live_inspect.rs`) previously relied on finding an existing `target/debug/blitz-host(.exe)`, risking silent execution against a stale binary if source changes were made without a manual rebuild.
   - Hardened both tests by adding `build_blitz_host_cli()`, which invokes `cargo build -p blitz-host` directly before CLI assertions execute.
   - All assertions are now guaranteed to run against the newly built CLI reflecting current workspace code.

---

## 3. Validation Actually Run

### Stage 1 — Windows Compile and Test Parity
1. **`blitz-host-transport` Check**:
   - `cargo check -p blitz-host-transport` -> Clean, 0 errors.
2. **`blitz-host` Binary Check & Test**:
   - `cargo check -p blitz-host` -> Clean, 0 errors.
   - `cargo test -p blitz-host` -> Clean, 0 errors.
3. **`oxidase-native-runner` Compilation**:
   - `cargo build` in `util/oxidase/crates/oxidase-native-runner` -> Compiled `oxidase-native-runner.exe` cleanly.
4. **Unix Cross-Platform Check**:
   - Verified on host/Unix copy: `cargo check -p blitz-host-transport` passes cleanly with the `.into_owned()` fix.

### Stage 2 — Explicit Named Pipe Targeting Proof (Post-Review Harness)
- Command: `cargo test -p blitz-host-transport --test live_cli_attach -- --nocapture`
- Result: **1 passed, 0 failed in 35.31s** (includes fresh in-harness `cargo build -p blitz-host`)
- Live Log Evidence:
  ```text
  running 1 test
     Compiling blitz-host-protocol v0.1.2 (D:\business\dioxus\util\blitz-host\crates\blitz-host-protocol)
     Compiling blitz-host-transport v0.1.2 (D:\business\dioxus\util\blitz-host\crates\blitz-host-transport)
     Compiling blitz-host-bridge v0.1.2 (D:\business\dioxus\util\blitz-host\crates\blitz-host-bridge)
     Compiling blitz-host v0.1.2 (D:\business\dioxus\util\blitz-host\crates\blitz-host)
      Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.15s

  Spawned live DebugServer host with PID 12628 on socket \\.\pipe\blitz-host-12628-1790320743718636200
  Testing 'blitz-host list' CLI over IPC...
  blitz-host list output:
  [
    {
      "protocolVersion": 1,
      "pid": 12628,
      "instanceId": "12628-1790320743718636200",
      "socketPath": "\\\\.\\pipe\\blitz-host-12628-1790320743718636200",
      "renderer": "test-live-host",
      "rendererVersion": "0.1.0"
    }
  ]

  Testing 'blitz-host inspect --pid 12628' CLI over Named Pipe...
  blitz-host inspect output:
  {
    "documentId": 1,
    "rootId": 100,
    "nodeCount": 2,
    "currentFrame": 1,
    "nodes": [
      { "id": 100, "tag": "div", "domId": "root", "role": "generic", "bounds": [0.0, 0.0, 800.0, 600.0], "children": [101] },
      { "id": 101, "parentId": 100, "tag": "button", "domId": "submit-btn", "role": "button", "text": "Submit", "bounds": [50.0, 50.0, 120.0, 40.0] }
    ]
  }

  Testing 'blitz-host inspect' auto-discovering single host...
  Testing 'blitz-host inspect \\.\pipe\blitz-host-12628-1790320743718636200' CLI over explicit Named Pipe path...
  Testing programmatic DebugClient::connect_target with TargetSelector::ExplicitPath...
  Testing unreachable explicit Named Pipe path rejection...
  Unreachable pipe correctly rejected: Explicit Named Pipe at '\\.\pipe\blitz-host-nonexistent-endpoint-test' is unreachable
  Test completed successfully!
  test test_live_cli_attach_and_inspect_named_pipe ... ok
  ```

### Stage 3 — Real Native Runner End-to-End Attach and Inspect Proof (Post-Review Harness)
- Command: `cargo test -p blitz-host-transport --test live_inspect -- --nocapture`
- Result: **1 passed, 0 failed in 10.06s**
- Live Log Evidence:
  ```text
  Starting oxidase-native-runner at "D:\\business\\dioxus\\util\\oxidase\\crates\\oxidase-native-runner\\target\\debug\\oxidase-native-runner.exe"...
  Spawned child process with PID: 8136
  [RUNNER stdout] [oxidase-native-runner] Launching Native Hosted Frame Test Runner
  [RUNNER stdout]   • Core Framework: Dioxus 0.7.10
  [RUNNER stdout]   • Render Engine : Blitz 0.3.0-beta.2 (Vello GPU)
  [RUNNER stdout]   • Window Host   : Winit 0.31 via dioxus-native
  [RUNNER stdout] [blitz-host] Local debug control server initialized
  [RUNNER stdout]   • Socket    : \\.\pipe\blitz-host-8136-...
  Successfully connected to live host with active document via PID 8136 on attempt #3
  Host Descriptor:
    • PID       : 8136
    • Socket    : \\.\pipe\blitz-host-8136-1790325430...
    • Instance  : 8136-1790325430...
    • Renderer  : oxidase-native-runner.exe
  Dispatching click action to button node #4294967417...
  Sending settle request for 2 frames...
  State change verified: initial text "Click to Test Event" -> "Clicked 1 times"
  Testing settle_until helper waiting for 'Clicked 2 times'...
  Focus action on input node #4294967413 -> focus indicator rendered
  SetValue action: "Hello from blitz-host live test!" -> typed text reactivity verified
  Second SetValue: "Continuous reactivity 42" -> verified
  Keyboard lane: Enter, Space, Shift+Tab, Ctrl+A + 'K', Backspace, Escape -> all verified
  Mouse / pointer lane: hover on #mouse-test-card, down, up, drag sequence -> verified
  Wheel scroll: delta_y=45 -> Scroll Y: 45 verified
  Visual capture: full window (800x600 PNG, 136,891 bytes) and subtree node capture -> verified
  CLI binary inspection over Named Pipe with out-of-process process execution -> verified
  CSS selector targeting: querySelector click, focus, set-value, move, capture -> verified
  LIVE ATTACH, CLICK, FOCUS, SET_VALUE, SUBTREE CAPTURE, CLI PROOF PASSED 100%!
  test test_live_native_runner_attach_and_inspect ... ok
  ```

### Stage 4 — Full Transport Test Suite (Post-Review Windows Rerun)
- Command: `cargo test -p blitz-host-transport`
- Result: **8 passed, 0 failed in 46.26s total**
  - Unit tests (`src/lib.rs`): 6 passed (0.63s)
  - CLI integration (`tests/live_cli_attach.rs`): 1 passed (35.31s, including CLI workspace build)
  - Live inspect harness (`tests/live_inspect.rs`): 1 passed (10.06s, Real Native Host)

### Distinction Between Mock vs Real Native Host Proof
1. **Mock Host Attach (`live_cli_attach.rs`)**:
   - **Fully proven** on Windows. Verifies CLI host listing, PID targeting, single-host auto-discovery, and explicit Named Pipe resolution and rejection.
2. **Explicit Named Pipe Targeting**:
   - **Fully proven** end-to-end on Windows. Both the CLI binary and programmatic client accept `\\.\pipe\...`, successfully connect, exchange typed JSON-RPC requests, and reject unreachable pipe targets with clean error messages.
3. **Real Native Runner Runtime Attach (`live_inspect.rs`)**:
   - **Fully proven** on Windows against real `oxidase-native-runner.exe`. Exercises the complete 14-step inspection, state mutation, keyboard, pointer, wheel, drag, subtree capture, and CLI targeting suite against a real native OS window and Vello GPU pipeline.

---

## 4. Final Verdict

**Windows transport lane hardened**

### Rationale
- **Gap A Closed**: The macOS hardcoded runner path in `live_inspect.rs` has been eliminated. The harness dynamically resolves `oxidase-native-runner.exe` on Windows, cleanly drains child process pipes, waits for window/document mount, and executes the entire 14-step live inspection suite against the real runner with 100% success.
- **Gap B Closed**: Explicit Windows Named Pipe targeting (`\\.\pipe\...`) is real and proven end-to-end across CLI argument parsing, device-namespace discovery without filesystem `.exists()` checks, programmatic client connection, and dead pipe rejection.
- **Post-Review Integrity**: Unix compile regression fixed (`.into_owned()`), and CLI test harness hardened (`build_blitz_host_cli()`) so no stale binary can ever be tested.
- **Full Verification**: All 8 tests across unit, integration, and real native runner attach suites pass with zero errors on Windows MSVC.

### Structural Recommendation
- The gap-closure brief outlined in `instruction-win.md` is now **fulfilled and complete**.
- Any future Windows work should treat this hardened baseline as the verified reference standard rather than reopening previous gap-closure tasks.
