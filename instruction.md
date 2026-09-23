# Worker Instruction: Remove `--debug-control` and Make `blitz-host` Always Available in the Feature-Enabled Dev Lane

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The previous work established:

1. `blitz-host` as a separate crate family
2. `oxidase` as the ergonomic integration boundary
3. a canonical cross-host example
4. a native-specific proof harness

The next direction is now fixed:

> **in the development lane, if `blitz-host` is compiled in, access through `blitz-host` should just be available — no `--debug-control` runtime flag required**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Remove the runtime `--debug-control` / `BLITZ_DEBUG_CONTROL` gating model from the current development/integration lane.

The intended model is now:

1. **compile-time opt-in** via the `blitz-host` feature
2. **always available at runtime** inside that feature-enabled development lane

In other words:

> if an app/example/harness is built with `oxidase` + `features = ["blitz-host"]`, then `blitz-host` access should be active by default in that dev lane without a separate runtime enable flag.

---

## 2. Required Direction

### A. Remove runtime flag dependence from the current dev story

The current `--debug-control` / `BLITZ_DEBUG_CONTROL` gating should be removed from the active development workflow.

That means cleaning up code and docs that currently imply:

1. feature compiled in
2. but still a second runtime switch is needed just to make local attach possible

That is no longer the desired UX.

### B. Keep compile-time opt-in

This does **not** mean always-on for all builds everywhere.

The boundary remains:

1. build with `blitz-host` feature → debug/control access is available
2. build without `blitz-host` feature → no debug/control integration

So the feature is still the opt-in.

What is being removed is the **extra runtime toggle** in the normal development lane.

---

## 3. What This Means in Practice

For the current `oxidase` ecosystem story, the desired development model becomes:

```toml
oxidase = { git = "...", tag = "...", features = ["native", "blitz-host"] }
```

and then:

```rust
#[oxidase::main]
fn main() {
    dioxus::launch(App);
}
```

with no further `--debug-control` requirement in ordinary development usage.

If the feature is present, the control plane should be available.

---

## 4. Required Cleanup Targets

You must remove or rewrite the runtime-flag model across the current lane where appropriate.

At minimum, inspect and update:

1. `oxidase` integration code
2. `blitz-host` host helpers
3. canonical `cross_host` example
4. `oxidase-native-runner`
5. `blitz-host` CLI/help/docs/examples/results that instruct users to pass `--debug-control`

### Important note

If you keep any runtime switch at all, it must be for a different reason than merely “make blitz-host available in development.”

For the current intended dev lane, feature presence should be enough.

---

## 5. Canonical Story After This Change

You must make the story explicit and consistent:

### Canonical cross-host example

If built with `blitz-host` feature:

1. Web/native example runs normally
2. Native debug attach is already available
3. no extra `--debug-control` argument is needed

### Native proof harness

If built with `blitz-host` feature:

1. the harness should already be attachable
2. tests should not require an extra runtime enable flag merely to expose the control plane

---

## 6. What Not to Break

Do not break:

1. the `blitz-host` / `oxidase` boundary
2. the canonical cross-host example
3. the native proof harness
4. attach / inspect / click / settle / changed-state proof

The goal is to simplify enablement, not reduce functionality.

---

## 7. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/README.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/src/launch.rs`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/src/prelude.rs`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-macro/src/main_macro.rs`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/examples/cross_host/main.rs`
7. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/src/main.rs`
8. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/`
9. tests that currently spawn or launch with `--debug-control`

---

## 8. Validation You Must Run

Run the smallest commands that prove the new always-available dev-lane story is real.

At minimum:

1. validate the canonical `cross_host` example in the feature-enabled native lane **without** `--debug-control`
2. validate the native proof harness attach path **without** `--debug-control`
3. run the `blitz-host` test suite after the change
4. confirm attach / inspect / click / settle / changed-state proof still works

If Web/native example validation is part of the current slice, keep both honest.

If markdown files are edited, validate them.

---

## 9. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what runtime gating was removed
2. what compile-time gating remains
3. how the example and harness are now activated in the feature-enabled lane
4. what commands were used to prove attachability without `--debug-control`
5. what still remains unresolved

---

## 10. Final Verdict Rule

You may report **Implemented and simplified** only if:

1. the current dev lane no longer needs `--debug-control` merely to make `blitz-host` available
2. feature-enabled builds are attachable by default in the intended lane
3. the example and runner stories remain honest and working
4. the proof path still works end to end

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make `blitz-host` availability automatic in the feature-enabled development lane, instead of requiring an extra runtime opt-in flag.
