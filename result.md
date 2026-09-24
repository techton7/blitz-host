# Result: Host Target Disambiguation & Shareable Ambiguity Guard

## 1. Current Repo Facts

1. **`blitz-host` Control Plane Baseline**:
   - `blitz-host` provides an out-of-process control plane and live DOM inspection harness for Blitz and Dioxus Native desktop applications over local Unix Domain Sockets (UDS).
   - Core interactive and proof lanes are complete: UI-thread synchronization, deterministic VSync settlement (`settle(n)`), visual full-window and cropped capture (`-o <PATH>`), compound keyboard shortcut dispatch, mouse namespace hierarchy, and CSS selector targeting (`ElementTarget`).

2. **Previous Target Selection Model**:
   - Prior to this pass, `TargetSelector::Auto` silently picked the first (newest modified) host descriptor from `list_hosts()`.
   - When multiple live Blitz instances were running concurrently (e.g. background runners, multiple test suites, or stale processes), `blitz-host` commands without `--pid` silently guessed and dispatched actions to an arbitrary host.
   - This introduced non-deterministic behavior and silent mis-targeting into automated and agentic test workflows.

3. **Core Scope Boundary**:
   - As specified in `instruction.md`, runtime scripting (Rhai, `eval`, `run`, REPL) remains deferred to the cross-host `oxidase` layer.
   - Same-process multi-window routing and speculative target extensions (e.g. `--instance`) remain out of scope for this pass.

---

## 2. What I Changed

1. **Transport Layer Ambiguity Guard (`crates/blitz-host-transport/src/discovery.rs`)**:
   - Implemented `resolve_target_from_hosts(selector: &TargetSelector, hosts: Vec<HostDescriptor>) -> io::Result<HostDescriptor>` as the single source of truth for host disambiguation.
   - Replaced silent guessing in `discover_target` with the deterministic policy:
     - **0 hosts**: Returns `io::ErrorKind::NotFound` (`"No active blitz-host instances found in $TMPDIR/blitz-host"`).
     - **1 host**: Returns `Ok(host)`, allowing implicit auto-discovery for zero-friction development.
     - **2+ hosts**: Rejects auto-discovery with `io::ErrorKind::InvalidInput`, reporting all competing PIDs:
       `"Multiple active Blitz hosts detected (PIDs: [...]). Target is ambiguous: please specify --pid <PID> (or connect_pid) to target a specific host."`
   - Explicit PID targeting (`TargetSelector::Pid(target_pid)`) deterministically filters for the matching PID regardless of the total number of running instances.

2. **CLI Connection Plumbing Consolidation (`crates/blitz-host/src/bin/blitz-host.rs`)**:
   - Created a centralized, shareable connection helper:
     ```rust
     fn connect_cli_client(selector: &TargetSelector) -> DebugClient {
         match DebugClient::connect_target(selector) {
             Ok(c) => c,
             Err(e) => {
                 eprintln!("Error connecting to Blitz host: {e}");
                 eprintln!("Use 'blitz-host list' to inspect available hosts.");
                 std::process::exit(1);
             }
         }
     }
     ```
   - Separated argument parsing (`determine_selector(subargs) -> TargetSelector`) from connection execution (`connect_cli_client(&selector)`).
   - Replaced repetitive 8-line connection/error boilerplate across all 11 subcommand execution paths (`handle_move_command`, `handle_down_command`, `handle_up_command`, `handle_wheel_command`, `handle_drag_command`, `handle_click_command`, `inspect`, `capture`, `focus`, `set-value`, and `key`).
   - Standardized exit code 1 with actionable stderr diagnostics directing users to `blitz-host list` and `--pid <PID>`.

3. **Transport Unit Test Suite (`crates/blitz-host-transport/src/discovery.rs`)**:
   - Added unit tests directly validating the disambiguation matrix:
     - `test_resolve_target_auto_zero_hosts_returns_not_found`: Proves 0 hosts returns `NotFound`.
     - `test_resolve_target_auto_single_host_succeeds`: Proves 1 host succeeds without `--pid`.
     - `test_resolve_target_auto_multiple_hosts_blocks_with_ambiguity`: Proves 2+ hosts fail fast with `InvalidInput` and list competing PIDs.
     - `test_resolve_target_pid_disambiguation_succeeds`: Proves explicit `--pid` resolves correctly among multiple competing hosts.
     - `test_resolve_target_pid_missing_returns_not_found`: Proves explicit non-existent PID returns `NotFound`.

4. **Preserved Deferred Scope**:
   - Runtime scripting (Rhai, `eval`, `run`, REPL) remains 100% out of scope and deferred.
   - Instance ID targeting (`--instance`) and multi-window routing remain deferred.

---

## 3. Validation Actually Run

1. **Discovery Unit Tests (`crates/blitz-host-transport`)**:
   ```bash
   cargo test -p blitz-host-transport --lib
   ```
   - Output: 6 passed; 0 failed; finished in 0.01s.
   - All 5 new ambiguity guard tests passed cleanly alongside the transport roundtrip test.

2. **Live Multi-Host CLI Disambiguation Proof**:
   - Spawned live `oxidase-native-runner` instance (PID 89303).
   - Spawned second concurrent `oxidase-native-runner` instance (PID 24713).
   - Verified `blitz-host list`:
     ```json
     [
       { "pid": 24713, "renderer": "oxidase-native-runner", ... },
       { "pid": 89303, "renderer": "oxidase-native-runner", ... }
     ]
     ```
   - **Ambiguity Guard Verification**: Ran `blitz-host inspect "#test-input"` without `--pid`.
     - Output: Exited with code `1` and printed:
       ```text
       Error connecting to Blitz host: Multiple active Blitz hosts detected (PIDs: [24713, 89303]). Target is ambiguous: please specify --pid <PID> (or connect_pid) to target a specific host.
       Use 'blitz-host list' to inspect available hosts.
       ```
   - **Disambiguation with `--pid` Verification**:
     - `blitz-host inspect "#test-input" --pid 89303`: Succeeded (exit code `0`), returned subtree from PID 89303.
     - `blitz-host inspect "#test-input" --pid 24713`: Succeeded (exit code `0`), returned subtree from PID 24713.
   - **Post-Cleanup Auto-Discovery**:
     - Terminated PID 24713.
     - Ran `blitz-host inspect "#test-input"` without `--pid`.
     - Succeeded (exit code `0`), automatically attaching to the remaining single host (PID 89303).
   - **Zero Hosts Verification**:
     - Terminated PID 89303.
     - Ran `blitz-host inspect "#test-input"`.
     - Exited with code `1` and printed:
       ```text
       Error connecting to Blitz host: No active blitz-host instances found in $TMPDIR/blitz-host
       Use 'blitz-host list' to inspect available hosts.
       ```

3. **Full Workspace Integration Suite (`crates/blitz-host-transport/tests/live_inspect.rs`)**:
   ```bash
   cargo test
   ```
   - 5 bridge unit tests passed.
   - 2 protocol unit tests passed.
   - 6 transport unit tests passed.
   - 14 live native E2E test steps passed in 10.83s with zero failures.

4. **Markdown Structure Verification**:
   ```bash
   bash /Users/gohyeoncheol/.gemini/config/skills/verify-markdown/bin/verify-markdown.sh result.md --require-frontmatter false
   bash /Users/gohyeoncheol/.gemini/config/skills/verify-markdown/bin/verify-markdown.sh handoff.md --require-frontmatter false
   ```

---

## 4. Final Verdict

**Implemented and clarified**

1. Implicit host selection works only in the single-host case (`len == 1`).
2. Ambiguous multi-host auto-discovery fails fast and cleanly with exit code `1` and actionable stderr guidance (`len >= 2`).
3. `--pid <PID>` deterministically resolves and targets specific hosts when multiple instances exist.
4. The ambiguity guard is implemented in the transport layer (`discovery::resolve_target_from_hosts`), guaranteeing single-source-of-truth protection for both CLI and programmatic Rust API consumers.
5. All 14 existing native control-plane and visual proof test steps continue to pass without regression.
