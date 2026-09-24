# Architecture Spec & Handoff: Host Target Disambiguation & Shareable Ambiguity Guard

## 1. Executive Summary & Core Decisions

This specification defines the architectural boundary for target selection and discovery within `blitz-host`, addressing the trade-off between stateless CLI ergonomics and deterministic test safety:

1. **Deterministic Target Disambiguation Model (Strict Ambiguity Guard)**:
   - **Single Host Active (`len == 1`)**: Auto-discovery succeeds without requiring `--pid`, providing zero-friction development ergonomics.
   - **Multiple Hosts Active (`len >= 2`)**: Silent guessing (such as picking the newest process) is **strictly prohibited**. The transport layer rejects ambiguous auto-discovery and fails fast (Exit Code 1), requiring the caller to specify `--pid <PID>` (or an explicit socket path).
   - **Explicit Target (`TargetSelector::Pid` / `--pid <PID>`)**: Always connects deterministically to the designated host regardless of how many other instances are running.
2. **Single Source of Truth in Transport Layer (`crates/blitz-host-transport`)**:
   - The ambiguity guard is implemented directly in `discovery::discover_target(&TargetSelector)`, guaranteeing that both the CLI binary and programmatic Rust API callers (`DebugClient::connect_discovered`) benefit from identical safety guarantees.
3. **Consolidated CLI Connection Plumbing (`crates/blitz-host/src/bin/blitz-host.rs`)**:
   - Subcommand connection boilerplate across `inspect`, `capture`, `focus`, `set-value`, `mouse *`, `key`, and `settle` is consolidated into a single shared helper `connect_cli_client(subargs: &[String]) -> DebugClient`.
4. **Re-affirmed Existing Invariants**:
   - **Native Control Focus**: Out-of-process UDS control plane for live Blitz windows (inspect, action dispatch, deterministic VSync settlement, visual capture).
   - **CSS Selector Targeting**: Polymorphic element targeting (`ElementTarget`: numeric ID or CSS selector via Stylo `query_selector`) remains fully active and proven.
   - **Deferred Runtime Scripting**: Dynamic scripting (Rhai, `eval`, `run`, REPL) remains strictly deferred to the cross-host `oxidase` layer.

---

## 2. Problem Analysis: Stateless CLI vs. Implicit Target Guessing

### 2.1 The Stateless CLI Dilemma
CLI commands in `blitz-host` are stateless, one-shot processes spawned by human developers, automated test harnesses, or AI coding agents:
```bash
blitz-host mouse click "#test-interaction-button"
blitz-host set-value "#test-input" "Hello"
```

In early iterations, two competing models were considered:
1. **Mandatory Explicit PID (`--pid` Required on Every Command)**:
   - *Advantage*: Completely deterministic.
   - *Disadvantage*: Severe ergonomics degradation during everyday local development. Developers must run `blitz-host list`, copy the PID, and append `--pid <PID>` to every single CLI invocation.
2. **Unconditional Auto-Discovery (Current Implementation)**:
   - *Advantage*: Zero friction when running a single window.
   - *Disadvantage*: When multiple Blitz applications, background runners, or stale orphaned processes exist simultaneously, the system silently picks the newest modified descriptor (`hosts.into_iter().next()`). This causes **silent mis-targeting**—actions intended for Window A are executed on Window B, causing flaky integration tests and hard-to-diagnose UI mutations.

### 2.2 The Solution: The `adb`-Style Ambiguity Guard
We adopt the proven industry-standard model used by tools like Android Debug Bridge (`adb`):
- When exactly **one** target is available, allow implicit targeting because there is zero ambiguity.
- When **two or more** targets are available, reject implicit targeting immediately with an actionable error message demanding `--pid`.

---

## 3. Architecture & Shareable Logic Design

```text
┌────────────────────────────────────────────────────────┐
│                   blitz-host CLI                       │
│    (inspect, capture, focus, set-value, mouse, key)    │
└──────────────────────────┬─────────────────────────────┘
                           │ 1) connect_cli_client(subargs)
                           ▼
┌────────────────────────────────────────────────────────┐
│              blitz-host-transport                      │
│            DebugClient::connect_target                 │
└──────────────────────────┬─────────────────────────────┘
                           │ 2) discover_target(Auto)
                           ▼
┌────────────────────────────────────────────────────────┐
│        discovery.rs (Ambiguity Guard Seam)             │
│   • len == 0 : NotFound ("No live host")               │
│   • len == 1 : Ok(host) (Single host pass-through)     │
│   • len >= 2 : Error ("Multiple hosts, specify --pid") │
└────────────────────────────────────────────────────────┘
```

### 3.1 Transport Layer Seam (`crates/blitz-host-transport/src/discovery.rs`)

The core decision logic resides in `discover_target`:

```rust
pub fn discover_target(selector: &TargetSelector) -> io::Result<HostDescriptor> {
    match selector {
        TargetSelector::ExplicitPath(path) => {
            // Unchanged: connect to explicit descriptor or socket path
            ...
        }
        TargetSelector::Pid(target_pid) => {
            // Unchanged: explicitly lookup specific PID
            let hosts = list_hosts()?;
            hosts
                .into_iter()
                .find(|d| d.pid == *target_pid)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("No live, reachable Blitz host found with PID {target_pid}"),
                    )
                })
        }
        TargetSelector::Auto => {
            let hosts = list_hosts()?;
            match hosts.len() {
                0 => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "No live, reachable Blitz host found via auto-discovery. Ensure a Blitz application is running with `blitz-host` enabled.",
                )),
                1 => Ok(hosts.into_iter().next().unwrap()),
                _ => {
                    let pids: Vec<u32> = hosts.iter().map(|h| h.pid).collect();
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Multiple active Blitz hosts detected (PIDs: {pids:?}). \
                             Target is ambiguous: please specify --pid <PID> (or connect_pid) to target a specific host."
                        ),
                    ))
                }
            }
        }
    }
}
```

### 3.2 CLI Centralized Connection Helper (`crates/blitz-host/src/bin/blitz-host.rs`)

All CLI subcommands currently duplicate the following 8-line connection snippet:
```rust
// Redundant pattern across 7+ subcommands
let mut client = match DebugClient::connect_target(&selector) {
    Ok(c) => c,
    Err(e) => {
        eprintln!("Error connecting to Blitz host: {e}");
        eprintln!("Make sure a Blitz host is running with `blitz-host` enabled.");
        eprintln!("Use 'blitz-host list' to inspect available hosts.");
        std::process::exit(1);
    }
};
```

This will be replaced with a single shareable helper function:

```rust
fn connect_cli_client(subargs: &[String]) -> DebugClient {
    let selector = determine_selector(subargs);
    match DebugClient::connect_target(&selector) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error connecting to Blitz host: {e}");
            eprintln!("Use 'blitz-host list' to inspect available hosts.");
            std::process::exit(1);
        }
    }
}
```

When multiple hosts exist, the error output presented to the user is clean, precise, and actionable:
```text
Error connecting to Blitz host: Multiple active Blitz hosts detected (PIDs: [89303, 89312]). Target is ambiguous: please specify --pid <PID> (or connect_pid) to target a specific host.
Use 'blitz-host list' to inspect available hosts.
```

---

## 4. Invariant Matrix

| Scenario | Active Hosts Count | Flag / Selector | Result | Rationale |
|---|---|---|---|---|
| **Zero Hosts** | 0 | Auto | `Err(NotFound)` | Clean fail-fast with guidance to launch an application |
| **Zero Hosts** | 0 | `--pid 1234` | `Err(NotFound)` | Specific process does not exist |
| **Single Host** | 1 (PID 89303) | Auto | `Ok(Host 89303)` | 100% unambiguous; maximum DX ergonomics |
| **Single Host** | 1 (PID 89303) | `--pid 89303` | `Ok(Host 89303)` | Explicit match |
| **Multiple Hosts** | 2+ (PIDs 89303, 89312) | Auto | `Err(InvalidInput)` (Exit Code 1) | **Blocked**: Ambiguous target; prevents silent mis-targeting |
| **Multiple Hosts** | 2+ (PIDs 89303, 89312) | `--pid 89303` | `Ok(Host 89303)` | Resolved by explicit disambiguation flag |

---

## 5. Implementation Roadmap & Verification Plan

### Phase 1: Transport Ambiguity Guard
1. Update `discover_target` in `crates/blitz-host-transport/src/discovery.rs` to inspect `hosts.len()`.
2. Add unit tests in `crates/blitz-host-transport/src/discovery.rs`:
   - `test_discover_target_auto_single_host`: Verifies single descriptor succeeds.
   - `test_discover_target_auto_multiple_hosts_blocks`: Verifies multiple descriptors return `InvalidInput` with PID list.
   - `test_discover_target_pid_disambiguation`: Verifies `--pid` succeeds even when multiple descriptors exist.

### Phase 2: CLI Consolidation
1. Implement `connect_cli_client` in `crates/blitz-host/src/bin/blitz-host.rs`.
2. Refactor all subcommands (`inspect`, `capture`, `focus`, `set-value`, `mouse *`, `key`, `settle`) to use the helper.
3. Validate CLI exits with code `1` and prints clean diagnostics when ambiguous.

### Phase 3: Integration Verification
1. Run existing native E2E suite:
   ```bash
   cargo test --test live_inspect
   ```
2. Verify all 14 integration test steps pass cleanly without regression.
