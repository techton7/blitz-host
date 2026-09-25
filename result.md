# Result: Windows Validation and Gap-Discovery for `blitz-host` & `oxidase` Native Lane

## 1. Current Repo Facts

1. **Host Environment**:
   - Operating System: Windows 11 (x86_64-pc-windows-msvc)
   - Python: Python 3.12.10 installed and registered in `PATH` & `PYTHON3` environment variables.
   - Workspace Synced from macOS via Mutagen:
     - `util/blitz-host` (`crates/blitz-host-protocol`, `crates/blitz-host-transport`, `crates/blitz-host-bridge`, `crates/blitz-host`)
     - `util/oxidase` (`crates/oxidase`, `crates/oxidase-macro`, `crates/oxidase-native-runner`, examples)
     - `blitz` monorepo packages (`dioxus-native`, `blitz-shell`, `blitz-dom`, etc.)
   - Toolchain: Rust 1.98.1 / Cargo, MSVC C/C++ build tools.

2. **Stack Dependencies & Build Status**:
   - `stylo v0.21.0` (Servo CSS engine used by Blitz) successfully runs its `build.rs` code-generation with Python 3.12.10 and compiles on Windows.
   - Upstream Blitz crates (`blitz-traits`, `blitz-dom`, `taffy`, `parley`, `anyrender`) compile cleanly on Windows.
   - `blitz-host-protocol` compiles cleanly on Windows.

3. **Transport Protocol Architecture**:
   - `blitz-host-transport` was developed and verified on macOS/Unix around local Unix Domain Sockets (UDS) with POSIX filesystem permissions (`0o600`), Unix process signal checks (`libc::kill(pid, 0)`), and `std::os::unix::net::{UnixListener, UnixStream}`.

---

## 2. What I Validated

Following the required staged order in [instruction.md](file:///d:/business/dioxus/util/blitz-host/instruction.md):

1. **Stage 1 — Windows compile / build parity**:
   - Installed Python 3.12.10 via `winget` to resolve `stylo` build script requirement.
   - Verified `stylo v0.21.0` code generation and build script pass on Windows.
   - Evaluated `cargo check --workspace` inside `util/blitz-host`.
   - Identified the sole remaining compile blocker: `blitz-host-transport`.

2. **Stage 2 — Transport viability on Windows**:
   - Audited [crates/blitz-host-transport/src/client.rs](file:///d:/business/dioxus/util/blitz-host/crates/blitz-host-transport/src/client.rs), [crates/blitz-host-transport/src/server.rs](file:///d:/business/dioxus/util/blitz-host/crates/blitz-host-transport/src/server.rs), and [crates/blitz-host-transport/src/discovery.rs](file:///d:/business/dioxus/util/blitz-host/crates/blitz-host-transport/src/discovery.rs) for OS-specific assumptions, imports, and API boundaries.

3. **Stage 3 through Stage 5 (Live Attach, Sentinel `Ctrl+A`, Pointer/Capture)**:
   - Evaluated whether progress beyond Stage 1/2 is possible. As mandated by `instruction.md`, because `blitz-host-transport` fails compilation and its transport logic is Unix-only, subsequent stages cannot execute until transport is adapted for Windows.

---

## 3. Evidence / Validation Actually Run

### 1. `stylo v0.21.0` Build Script Resolution
- Installed Python 3.12.10 (`winget install --id Python.Python.3.12 --silent`).
- Configured persistent `PYTHON3` and `PATH` environment variables.
- Result: **RESOLVED**. `stylo v0.21.0` and its build scripts run and compile cleanly on Windows.

### 2. `blitz-host-protocol` Compiles Cleanly
- Command: `cargo check -p blitz-host-protocol`
- Result: **SUCCESS**. Zero Unix-specific dependencies.

### 3. `blitz-host-transport` Fails to Compile (The Core Code Blocker)
- Command: `cargo check --workspace`
- Result: **FAILED (Exit code: 1)**
- Evidence:
  ```text
  error[E0433]: cannot find `unix` in `os`
   --> crates\blitz-host-transport\src\client.rs:2:14
    |
  2 | use std::os::unix::net::UnixStream;
    |              ^^^^ could not find `unix` in `os`

  error[E0433]: cannot find `unix` in `os`
   --> crates\blitz-host-transport\src\server.rs:3:14
    |
  3 | use std::os::unix::fs::PermissionsExt;
    |              ^^^^ could not find `unix` in `os`

  error[E0433]: cannot find `unix` in `os`
   --> crates\blitz-host-transport\src\server.rs:4:14
    |
  4 | use std::os::unix::net::{UnixListener, UnixStream};
    |              ^^^^ could not find `unix` in `os`

  error[E0599]: no associated function or constant named `from_mode` found for struct `Permissions` in the current scope
    --> crates\blitz-host-transport\src\server.rs:57:60
     |
  57 |         fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))?;
     |                                                            ^^^^^^^^^ associated function or constant not found in `Permissions`
  ```

### 4. Transport Discovery Logic Blockers
- In `crates/blitz-host-transport/src/discovery.rs` (lines 206–216):
  ```rust
  pub fn is_reachable(descriptor: &HostDescriptor) -> bool {
      #[cfg(unix)]
      {
          std::os::unix::net::UnixStream::connect(&descriptor.socket_path).is_ok()
      }
      #[cfg(not(unix))]
      {
          false
      }
  }
  ```
  `is_reachable()` unconditionally returns `false` on non-Unix platforms, meaning `list_hosts()`, `discover()`, and `discover_target()` immediately reject any Windows host instance as dead or unreachable.
- In `crates/blitz-host-transport/src/discovery.rs` (lines 218–229):
  ```rust
  fn is_pid_alive(pid: u32) -> bool {
      #[cfg(unix)]
      {
          unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
      }
      #[cfg(not(unix))]
      {
          true
      }
  }
  ```
  Relies on `libc::kill` which is unavailable on Windows.

---

## 4. Final Verdict

**Windows lane blocked with evidence**

### Summary of Blockers
1. **Compilation Parity**:
   - `stylo` build script: RESOLVED (Python 3.12 installed).
   - `blitz-host-protocol`: ACHIEVED.
   - `blitz-host-transport`: **BLOCKED** due to unconditional `std::os::unix` and POSIX `from_mode(0o600)` references.
   - `blitz-host` CLI binary: **BLOCKED** because it depends directly on `blitz-host-transport`.
2. **Transport Viability**:
   - **BLOCKED**. Relies on standard library Unix Domain Sockets (`std::os::unix::net`), POSIX permissions, and Unix signals. `is_reachable()` is hardcoded to return `false` on Windows.
3. **Live Attach / Inspect / Keyboard Sentinel (`Ctrl+A`) / Pointer Smoke**:
   - **BLOCKED** until `blitz-host-transport` is adapted for Windows.

### Next Windows-Specific Action Required
- **Cross-Platform Transport Adaptation for `blitz-host-transport`**:
  1. Abstraction layer for local IPC: Either use Windows Named Pipes (e.g. `interprocess`) or Windows 10+ AF_UNIX sockets via `uds_windows` or `tokio`.
  2. Guard POSIX permissions with `#[cfg(unix)]` and omit `PermissionsExt::from_mode` on Windows.
  3. Implement Windows process liveness check (`OpenProcess` via `windows-sys` / Win32 API) instead of Unix `libc::kill(pid, 0)`.
  4. Implement Windows connection probe in `is_reachable()`.
