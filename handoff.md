# Architecture Spec & Handoff: `blitz-host` CLI Ergonomization & Deterministic JSON Protocol

## 1. Summary

This specification establishes the canonical CLI design and UX improvements for `blitz-host`:
1. **Always-On JSON Protocol**: Eliminates the `--json` flag; all subcommands return deterministic, machine-parseable JSON on `stdout`. Informational logs and status messages are directed to `stderr` or encapsulated within the JSON response envelope.
2. **Compound Key Syntax (`key`)**: Eliminates separate modifier flags (`--shift`, `--ctrl`, `--alt`, `--meta`). Replaces them with intuitive compound key strings (e.g. `cmd+a`, `command+shift+z`, `shift+tab`, `ctrl+c`) with case-insensitive tokenization and normalization.
3. **`mouse` Namespace Hierarchy**: Groups all pointer actions (`move`, `down`, `up`, `wheel`, `drag`) under the `blitz-host mouse` namespace, eliminating top-level namespace pollution while retaining backward-compatible aliases.
4. **Hierarchical `--help` Documentation**: Introduces dedicated help manuals for the `mouse` namespace (`blitz-host mouse --help`) and updates all subcommand manuals to reflect compound keys and default JSON output.

---

## 2. Core Architecture Invariants

### 1. Always-On JSON Output (`--json` Flag Elimination)
- **Zero Ambiguity**: Subcommands never switch between human ASCII tables/trees and JSON based on an optional flag.
- **Agent & Pipeline Native**: `stdout` is reserved strictly for valid JSON payloads. Tools like `jq`, AI agents, and CI pipelines can consume outputs directly without parsing errors.
- **Diagnostic Hygiene**: Any diagnostic logs (such as connection banners, VSync settle notifications) MUST be routed to `stderr` via `eprintln!` so that `stdout` contains only the valid JSON response envelope.
- **Unified JSON Response Envelopes**:
  - `list`: JSON array of `HostDescriptor` objects `[ { "pid": ..., ... }, ... ]`.
  - `inspect`: Pure `InspectResponse` JSON object.
  - `capture`: Compact JSON object with `outputPath` and dimensions (no massive base64 in stdout; image bytes written directly to the mandatory `-o` path):
    ```json
    {
      "success": true,
      "outputPath": "target/card.png",
      "width": 618,
      "height": 40,
      "format": "png",
      "nodeId": 4294967464,
      "bytes": 7443
    }
    ```
  - `click`, `focus`, `set-value`, `key`, `mouse *`: Pure `ActionResponse` JSON object:
    ```json
    {
      "success": true,
      "node_id": 4294967453,
      "message": "Dispatched synthetic click to node #4294967453"
    }
    ```

### 2. Compound Key Specification & Normalization (`key`)
- **No Separate Modifier Flags**: Flags `--shift`, `--ctrl`, `--alt`, `--meta`, `--cmd` are removed from `blitz-host key`.
- **Compound Delimiter**: Key expressions are split by `+` (e.g. `cmd+shift+a`, `"command+enter"`, `ctrl+v`, `alt+arrowup`).
- **Case-Insensitive Modifier Matching**:
  - `cmd`, `command`, `meta`, `super` -> `Modifiers::SUPER`
  - `shift` -> `Modifiers::SHIFT`
  - `ctrl`, `control` -> `Modifiers::CONTROL`
  - `alt`, `option`, `opt` -> `Modifiers::ALT`
- **Case-Insensitive Key Normalization**:
  - Named navigation/action keys are parsed case-insensitively (`tab`, `enter`, `return`, `esc`, `escape`, `backspace`, `del`, `delete`, `space`, `arrowleft`, `left`, `arrowright`, `right`, `arrowup`, `up`, `arrowdown`, `down`).
  - Single characters (`a`..`z`, `0`..`9`) are preserved with their character values and mapped to standard virtual keycodes.

### 3. `mouse` Namespace Hierarchy
- **Canonical Syntax**:
  - `blitz-host mouse move [NODE_ID] [--x <X> --y <Y>] [--pid <PID>]`
  - `blitz-host mouse down [NODE_ID] [--button <BUTTON>] [--x <X> --y <Y>] [--pid <PID>]`
  - `blitz-host mouse up [NODE_ID] [--button <BUTTON>] [--x <X> --y <Y>] [--pid <PID>]`
  - `blitz-host mouse wheel [NODE_ID] --dy <DY> [--dx <DX>] [--pid <PID>]`
  - `blitz-host mouse drag <FROM_ID> <TO_ID> [--pid <PID>]`
- **Top-Level Cleanliness**: Top-level `blitz-host --help` lists `mouse` as a compound namespace rather than 5 separate mouse commands.
- **Backward Compatibility**: Direct calls to `blitz-host move`, `down`, `up`, `wheel`, `drag` remain supported as aliases to prevent breaking existing scripts.

### 4. Hierarchical Help (`--help` / `-h`)
- Running `blitz-host mouse --help` or `blitz-host mouse -h` displays the consolidated manual for all mouse capabilities.
- Running `blitz-host mouse <subcommand> --help` displays the subcommand-specific manual.
- Subcommand manuals reflect the removal of `--json` and the updated compound key syntax.

---

## 3. Canonical Public CLI Surface

### Top-Level Overview (`blitz-host --help`)
```text
blitz-host: Out-of-process control plane for live Blitz desktop applications.

USAGE:
    blitz-host <SUBCOMMAND>

SUBCOMMANDS:
    list [OPTIONS]                 List active, reachable Blitz desktop host processes (JSON)
    inspect [OPTIONS]              Inspect live window semantic DOM & layout tree (JSON)
    capture [NODE_ID] -o <PATH>    Capture live visual screenshot or node crop (mandatory -o, JSON)
    click <NODE_ID> [OPTIONS]      Dispatch synthetic click to element (auto-settles, JSON)
    focus <NODE_ID> [OPTIONS]      Focus target element (auto-settles, JSON)
    set-value <NODE_ID> <VALUE>    Set text value of an input element (auto-settles, JSON)
    key <KEY_SPEC> [OPTIONS]       Dispatch key event (e.g. cmd+a, shift+tab, enter) (JSON)
    mouse <SUBCOMMAND> [OPTIONS]   Pointer and mouse interactions (move, down, up, wheel, drag)

OPTIONS:
    -h, --help                     Print help information
    -V, --version                  Print version information
```

### `key` Subcommand Syntax
```bash
# Activation & navigation
blitz-host key enter
blitz-host key tab
blitz-host key shift+tab
blitz-host key escape

# Editing & Shortcuts
blitz-host key cmd+a
blitz-host key command+shift+z
blitz-host key ctrl+c
blitz-host key backspace --node 4294967405
```

### `mouse` Namespace Syntax
```bash
# Namespace help
blitz-host mouse --help

# Pointer actions
blitz-host mouse move 4294967464
blitz-host mouse move --x 150 --y 200
blitz-host mouse down 4294967464 --button right
blitz-host mouse up 4294967464
blitz-host mouse wheel 4294967473 --dy 50
blitz-host mouse drag 4294967464 4294967449
```

---

## 4. Implementation Inventory

1. **`crates/blitz-host/src/bin/blitz-host.rs`**:
   - Update `print_main_help`: list `mouse` namespace, remove top-level clutter.
   - Add `print_mouse_namespace_help`: consolidated guide for `move`, `down`, `up`, `wheel`, `drag`.
   - Update `print_key_help`: document compound key syntax (`cmd+a`, `command+shift+z`).
   - Remove `--json` flags across all subcommand parsers; make JSON output unconditional on `stdout`.
   - Re-route interactive diagnostic messages (`Connecting...`, `Synchronizing frames...`) to `eprintln!`.
   - Implement `mouse` subcommand dispatcher routing to `move`, `down`, `up`, `wheel`, `drag`.
   - Retain top-level match arms for `move`, `down`, `up`, `wheel`, `drag` as aliases.
2. **`crates/blitz-host-protocol` / `blitz/packages/dioxus-native-dom/src/events.rs`**:
   - Enhance `parse_key_str` to support `command`, `cmd`, `super`, `meta`, `ctrl`, `shift`, `alt`, `opt` case-insensitively.
   - Add case-insensitive normalization for named keys (`tab`, `enter`, `escape`, `backspace`, `delete`, `space`, `arrow*`).

---

## 5. Verification & Proof Path

1. **Compilation & Clippy**:
   ```bash
   cargo check --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml
   ```
2. **CLI Output Determinism**:
   - `blitz-host list` produces valid JSON parseable by `jq`.
   - `blitz-host inspect` produces valid JSON parseable by `jq`.
   - `blitz-host key "cmd+a"` dispatches with `Modifiers::SUPER` and key `'a'`.
   - `blitz-host key "shift+tab"` traverses focus backwards.
   - `blitz-host mouse move 4294967464` triggers hover via namespace.
   - `blitz-host mouse --help` displays clean namespace documentation.
3. **Automated Rust Test Suite**:
   ```bash
   cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture
   ```
