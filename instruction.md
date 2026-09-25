# Worker Instruction: Validate the Current `blitz-host` / `oxidase` Native Lane on Windows

You are working against the Windows-synced copy of the repository.

This is a **Windows validation and gap-discovery pass**, not a general feature implementation pass.

The goal is to determine how much of the current macOS/Unix-proven `blitz-host` + `oxidase` native lane already works on Windows, and where the first real blockers are.

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I validated**
3. **evidence / validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Validate the current native control-plane stack on Windows in the most pragmatic order:

1. **compile/build parity**
2. **transport viability**
3. **minimal live attach proof**
4. **modifier sentinel (`Ctrl+A`)**
5. **pointer / capture smoke proof**

This pass is about establishing what is already true on Windows and what is blocked.

---

## 2. Highest-Priority Question

The most important question is:

> **does the current `blitz-host` transport and host integration even work on Windows, or is the current implementation too Unix-specific?**

That must be answered before we care about deeper workflow polish.

You must look for concrete blockers such as:

1. `std::os::unix::*` usage
2. Unix Domain Socket assumptions
3. POSIX permission assumptions (`0o600`, `0o700`)
4. path, file, or process liveness logic that is Unix-only

If the transport itself blocks on Windows, say so plainly and stop pretending later tests are meaningful.

---

## 3. Validation Order You Must Follow

### Stage 1 — Windows compile / build parity

Start here.

At minimum, try to build/check:

1. `util/blitz-host`
2. the canonical `cross_host` example in the `oxidase` crate
3. `oxidase-native-runner`

If the stack does not compile on Windows, capture the exact blocker and stop expanding the scope.

### Stage 2 — Transport viability

If the build succeeds, determine whether the transport is actually usable on Windows.

Questions to answer:

1. does host descriptor publication work?
2. does discovery work?
3. does the current local IPC model work?
4. if not, is the blocker specifically UDS/Unix-only or something else?

### Stage 3 — Minimal live attach proof

If transport is viable, prove the smallest useful native attach workflow on Windows:

1. launch a native target (`cross_host` and/or `oxidase-native-runner`)
2. discover / attach
3. inspect

You do not need every single feature first — establish the minimal proof that the lane is alive.

### Stage 4 — Modifier sentinel

If minimal attach works, verify the platform-specific modifier proof case:

1. `Ctrl+A` on Windows
2. overwrite / replace behavior after selection

This is the highest-value keyboard-specific parity check on Windows.

### Stage 5 — Pointer / capture smoke proof

If earlier stages pass, smoke-test:

1. hover / move
2. wheel / scroll
3. capture

This does not need to be a full exhaustive matrix unless the stack is already stable.

---

## 4. Scope Boundary

### Must do

1. validate what currently works on Windows
2. identify the first real blockers honestly
3. distinguish “does not compile” from “compiles but attach path fails” from “attach works but specific features fail”

### Must not do

1. do not paper over Windows blockers with speculation
2. do not treat macOS results as if they imply Windows parity
3. do not jump to implementing a Windows transport redesign unless the blocker is conclusively identified and tiny to fix

This pass is about evidence first.

---

## 5. Specific Areas to Inspect

At minimum inspect and/or validate:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/` code that assumes Unix transport/filesystem behavior
2. `crates/blitz-host-transport/src/discovery.rs`
3. `crates/blitz-host-transport/src/server.rs`
4. `crates/blitz-host/src/bin/blitz-host.rs`
5. `util/oxidase/crates/oxidase/examples/cross_host/`
6. `util/oxidase/crates/oxidase-native-runner/`

If you are operating only on the Windows copy, inspect the equivalent Windows-side synced paths there.

---

## 6. Validation You Must Run

Run the smallest commands that give real Windows evidence.

At minimum, attempt the Windows equivalents of:

1. `cargo check` / `cargo build` for `util/blitz-host`
2. `cargo build` or `cargo run` for the `cross_host` native example
3. `cargo build` or `cargo run` for `oxidase-native-runner`
4. if the host launches, `blitz-host list`
5. if attach works, `inspect`
6. if attach and inspect work, `Ctrl+A` sentinel
7. pointer / capture smoke proof if feasible

Be explicit about which stages were actually reached.

---

## 7. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. whether Windows compile/build parity was achieved
2. whether transport is viable on Windows
3. whether attach / inspect works
4. whether `Ctrl+A` works
5. whether pointer / capture smoke proof works
6. the first concrete blocker if the lane breaks
7. what the next Windows-specific action should be

---

## 8. Final Verdict Rule

You may report one of these:

1. **Windows lane works**
2. **Windows lane partially works**
3. **Windows lane blocked with evidence**

Your verdict must be based on the staged validation above, not assumptions.

The purpose of this pass is:

> establish the real Windows status of the current `blitz-host` / `oxidase` native lane before we decide what, if anything, needs Windows-specific redesign.
