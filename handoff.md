# Architecture Spec & Handoff: Control Plane Boundary — Native Selector Targeting & Deferred Runtime Scripting

## 1. Executive Summary & Confirmed Architectural Decision

This document establishes the confirmed architectural boundary for `blitz-host` and clarifies the separation between **native control-plane primitives** and **runtime scripting**:

1. **`blitz-host` Remains Focused on Native Primitives**:
   - `blitz-host` continues to serve exclusively as the local, out-of-process control plane and inspection harness for live Blitz windows (inspect, action dispatch, deterministic frame settlement, visual capture, and targeting).
2. **CSS Selector Targeting is Implemented and Proven**:
   - Selector-based targeting (`querySelector` backed by the native Stylo engine in `blitz_dom::BaseDocument`) is now fully implemented and verified across the protocol, bridge, client API, and CLI.
   - Callers can target elements using standard CSS selectors (e.g. `"#test-input"`, `"#submit-btn"`, `".todo-card"`) or direct numeric node IDs (`4294967402`).
   - Resolution executes on the live UI thread immediately before action dispatch, guaranteeing that actions operate on current live DOM state.
3. **Runtime Scripting Inside `blitz-host` Remains Deferred**:
   - Embedding a dynamic scripting engine (such as Rhai, Boa, or equivalent), adding `eval` / `run` subcommands, or implementing in-process DOM scripting wrappers inside `blitz-host` is **strictly deferred**.
   - `blitz-host` will not ship an embedded runtime scripting layer.
4. **Future Shared Scripting / Scenario Layer Belongs at `oxidase`**:
   - If a shared automation, E2E test runner, or declarative scenario scripting language is pursued in the future, it belongs at the **`oxidase` layer** as a cross-host abstraction.
   - At the `oxidase` level, a scenario runner can drive both the **Web backend** (via browser automation / WebDriver) and the **Native backend** (via `blitz-host` typed IPC) using unified test intent.

---

## 2. Architectural Boundary Analysis

### 2.1 The Role of `blitz-host`: Native Control Driver
`blitz-host` is designed as the native counterpart to the Chrome DevTools Protocol (CDP) or native WebDriver server. Its responsibilities are:
- Local Unix Domain Socket (UDS) transport and discovery (`~/.blitz-host/`).
- Main UI-thread synchronization and deterministic VSync frame settlement (`settle(n)`).
- Direct traversal of `blitz_dom::BaseDocument` for layout-measured semantic DOM inspection.
- Native CSS selector resolution via `blitz_dom::BaseDocument::query_selector` (Stylo) with bare ID fallback.
- Dispatching synthetic input events (`MouseEvent`, `KeyboardEvent`, `FocusEvent`) into `dioxus-native-dom`.
- High-fidelity visual rasterization and node crop capture to disk.

Embedding a scripting language (like Rhai) directly into `blitz-host` would create an architectural mismatch:
- It couples a specific scripting language into the low-level native driver.
- It fails to solve cross-platform verification, because the same script would not run against the Web target.
- It expands the attack surface and complexity of the native host bridge unnecessarily.

### 2.2 The Role of `oxidase`: Cross-Host Scenario Orchestration
`oxidase` is the cross-host application framework whose core mission is code and behavior parity across Web and Native (`crates/oxidase/examples/cross_host/`).

If an automation scripting language or high-level test scenario runner is built:
```text
┌────────────────────────────────────────────────────────┐
│              oxidase Scenario Runner                   │
│        (Shared Test Scenarios / Rhai / E2E)            │
└──────────────────────────┬─────────────────────────────┘
                           │
             ┌─────────────┴─────────────┐
             ▼                           ▼
   ┌───────────────────┐       ┌───────────────────┐
   │    Web Driver     │       │    blitz-host     │
   │ (Playwright / CDP)│       │ (Native UDS IPC)  │
   └───────────────────┘       └───────────────────┘
```
- A test scenario defined at the `oxidase` layer can execute identical assertions against both Web and Native builds.
- On the Web side, `oxidase` delegates to standard browser drivers (Playwright / CDP).
- On the Native side, `oxidase` issues typed IPC commands to `blitz-host`.
- This preserves `blitz-host` as a clean, lean native driver while delivering cross-host test automation.

---

## 3. Disentangling Selector Ergonomics from Scripting

The selector targeting milestone was completed without introducing a dynamic scripting engine.

### Completed Selector Targeting Architecture:
- **Polymorphic Target Parameter (`ElementTarget`)**: All targeting commands accept either numeric node IDs or CSS selector strings.
- **Direct Stylo Resolution**: Reuses `blitz_dom::BaseDocument::query_selector` directly inside `blitz-host-bridge` on the live UI thread immediately before action dispatch.
- **Pure Native Primitive**: Requires no scripting engine, no `eval`, and no dynamic runtime—it resolves an element against the live document before dispatching the existing typed `ActionRequest`.

### What Remains Strictly Deferred:
- In-process scripting engines (Rhai, Boa, QuickJS).
- Dynamic code evaluation (`blitz-host eval "<code...>"`).
- Script file execution (`blitz-host run <file.rhai>`).
- Scripting DOM object models and method bindings (`Element.prototype.click()`).
- REPL / interactive shell sessions.

---

## 4. Current Established State of `blitz-host`

The active `blitz-host` implementation is complete, stable, and 100% verified across all core interaction lanes:
1. **Always-On JSON Protocol**: All subcommands unconditionally output machine-parseable JSON on `stdout`. Diagnostic messages route to `stderr`.
2. **Strict `mouse` Namespace Hierarchy**: All pointer actions (`click`, `move`, `down`, `up`, `wheel`, `drag`) live strictly under `blitz-host mouse`. Top-level pointer aliases are rejected with exit code `1` (Zero Fallback).
3. **Capture Simplification**: Mandatory `-o <PATH>` for visual capture, writing PNG bytes directly to disk and outputting compact metadata-only JSON (zero base64 in the wire protocol).
4. **Symmetrical `[TARGET]` Targeting**: `blitz-host inspect [TARGET]` allows full-window or scoped subtree inspection by numeric node ID or CSS selector.
5. **Compound Key Syntax**: `blitz-host key cmd+a`, `command+shift+z`, `shift+tab` with case-insensitive tokenization.
6. **Live CSS Selector Targeting**: `blitz-host mouse click "#btn"`, `focus "#input"`, `set-value "#input" "val"`, `capture "#card" -o file.png`, `inspect "#card"` all resolve dynamically on the UI thread.

---

## 5. Next Steps & Implementation Directives

1. **Do not implement Rhai or any runtime scripting engine in `blitz-host`**.
2. Keep `blitz-host` focused on primitive stability, transport reliability, and deterministic native verification.
3. Anchor any future programmable scenario / scripting design discussions at the `oxidase` workspace layer.
