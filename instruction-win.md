# Worker Instruction (Windows): Harden the Remaining `blitz-host` Windows Gaps

You are working against the Windows-synced copy of the repository.

This is **not** the initial Windows transport adaptation pass anymore. The Named Pipe transport work already exists and `result-win.md` now claims the Windows transport lane is implemented.

Your job is to close the remaining honesty gaps so that the Windows story is not merely “transport compiles and mock attach works,” but has stronger end-to-end proof and fewer platform-specific rough edges.

Write all agent-facing reasoning in English.

Use this exact reporting structure in `result-win.md`:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Current Starting Point

The current Windows result already establishes these facts:

1. Windows Named Pipes are the chosen IPC backend.
2. `blitz-host-transport` was adapted away from Unix-only `std::os::unix` assumptions.
3. `live_cli_attach.rs` provides real transport-level proof that the CLI can discover and inspect a mock live host over Named Pipes.

However, two important gaps remain:

1. **`tests/live_inspect.rs` is not honest Windows proof yet**
   - It still hardcodes the macOS runner path (`/Volumes/.../oxidase-native-runner`).
   - On Windows it can return early instead of proving a real native attach path.
2. **Explicit `\\.\pipe\...` targeting is not yet proven end-to-end**
   - CLI selector parsing still mainly recognizes explicit descriptor/socket paths through `.json` / `.sock` suffixes.
   - `discover_target()` has a Windows Named Pipe branch, but the CLI and discovery path should be made provably correct for explicit pipe-path targeting.

This pass is about fixing those two gaps first.

---

## 2. Core Goal

Strengthen the Windows lane from:

> “transport implemented, with mock attach proof”

to:

> “transport implemented, explicit pipe targeting works, and the real native runner path is honestly testable on Windows”

The primary outcome should be stronger Windows proof, not architectural churn.

---

## 3. Required Work Areas

### A. Replace the macOS-only runner path in `tests/live_inspect.rs`

You must remove the hardcoded `/Volumes/.../oxidase-native-runner` dependency from:

- `util/blitz-host/crates/blitz-host-transport/tests/live_inspect.rs`

The test should locate the runner in a way that is actually usable on Windows.

Acceptable directions include:

1. manifest-relative workspace path resolution
2. environment-variable override
3. a reliable build/test-time binary location strategy

The result must be honest. Do not just swap in a different hardcoded Windows absolute path.

### B. Make explicit Named Pipe targeting real, not implied

Inspect and, if necessary, correct:

- `util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`
- `util/blitz-host/crates/blitz-host-transport/src/discovery.rs`

The goal is that an explicit Windows pipe path such as:

```text
\\.\pipe\blitz-host-<instance>
```

is handled as an explicit endpoint, not silently treated as auto-discovery or rejected by filesystem assumptions.

At minimum:

1. CLI selector parsing must correctly classify explicit Named Pipe targets
2. discovery logic must not rely on ordinary `Path::exists()` semantics for Named Pipes
3. explicit pipe targeting must be validated with a real test or smoke proof

### C. Keep the existing Windows transport behavior intact

Do not regress:

1. `blitz-host list`
2. `--pid` targeting
3. auto-discovery ambiguity guard
4. transport roundtrip over Named Pipes

---

## 4. Scope Boundary

### Must attempt

1. real Windows-usable runner resolution for `live_inspect.rs`
2. explicit Named Pipe target handling end-to-end
3. Windows validation proving the new behavior
4. update `result-win.md` with the narrowed, honest state

### May defer if needed

If full real-runner attach still cannot be proven after removing the path blocker, you may stop at the strongest honest partial state, but you must say exactly what still blocks runtime proof.

### Must not do

1. do not keep the old hardcoded macOS path and merely document it again
2. do not claim explicit Named Pipe targeting works without a dedicated proof
3. do not replace concrete Windows proof with more mock-only reassurance

---

## 5. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/tests/live_inspect.rs`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/tests/live_cli_attach.rs`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/src/discovery.rs`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result-win.md`

---

## 6. Validation Ladder

Follow this order:

### Stage 1 — compile and targeted test parity

At minimum:

1. `cargo check -p blitz-host-transport`
2. `cargo check -p blitz-host`
3. targeted tests you modified

### Stage 2 — explicit Named Pipe targeting proof

Prove one of these paths works on Windows:

1. CLI invocation against an explicit `\\.\pipe\...` endpoint
2. programmatic client explicit-path connect path exercised by a dedicated test

This proof must be explicit. Do not treat `--pid` or auto-discovery as a substitute.

### Stage 3 — real runner attach proof

If Stage 1/2 succeed far enough, prove the strongest real attach flow you can:

1. launch `oxidase-native-runner`
2. connect from `blitz-host`
3. inspect successfully

If this still fails, record the exact remaining blocker after the path-fix work.

---

## 7. Result File Requirement

Write the updated Windows-specific outcome to:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result-win.md`

The updated result must explicitly state:

1. whether `live_inspect.rs` became real Windows evidence
2. whether explicit `\\.\pipe\...` targeting now works
3. what proof is mock-only vs real native-host proof
4. what still fails, if anything

If you downgrade or keep the current verdict, explain why.

---

## 8. Final Verdict Rule

You may report one of these:

1. **Windows transport lane hardened**
2. **Windows transport lane partially hardened**
3. **Windows transport lane still has unproven edges**

Choose the verdict that matches the strongest evidence you actually have.

The purpose of this pass is:

> convert the current Windows result from “mostly credible with two important caveats” into a stronger, directly proven Windows story for explicit pipe targeting and real native-runner attach.
