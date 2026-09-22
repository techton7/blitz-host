# Worker Instruction: Finish the `oxidase` Boundary Move for `blitz-host` for Real

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The previous pass moved in the right direction, but it stopped halfway and overclaimed the result.

This pass must finish the job honestly.

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

The intended architecture is now fixed:

1. `blitz-host` remains a separate project/crate family
2. `oxidase` is the ergonomic integration boundary
3. `dioxus-native-dom` does **not** directly depend on `blitz-host`
4. an `oxidase` consumer with the `blitz-host` feature should be able to use ordinary:

```rust
#[oxidase::main]
fn main() {
    dioxus::launch(App);
}
```

without directly importing `blitz_host`, calling `blitz_host::init_if_debug(...)`, or wrapping RSX in `<BlitzHost>`.

That is the actual target.

---

## 2. What Was Wrong With the Previous Pass

The previous pass was not sufficient because:

1. `oxidase-native-runner` still directly imported `blitz_host`
2. it still directly called `blitz_host::init_if_debug(...)`
3. it still directly wrapped the app UI in `<BlitzHost>`
4. the result text claimed downstream consumers were free of direct `blitz-host` usage, which was false
5. the result text also implied `oxidase-v0.1.4` already represented this state, which was false because `util/oxidase` was ahead of the `oxidase-v0.1.4` tag

This pass must correct the code **and** the truthfulness.

---

## 3. Required Outcome

### A. Real ergonomic boundary

Make the ergonomic boundary real, not aspirational.

That means:

1. a host/runnner using `oxidase` with `features = ["native", "blitz-host"]`
2. and using ordinary `#[oxidase::main]`

should not need any explicit `blitz_host::*` calls in application code.

### B. Remove direct `blitz-host` consumer burden from the runner

In `oxidase-native-runner`, remove direct app-level usage of:

1. `use blitz_host::...`
2. `blitz_host::init_if_debug(...)`
3. `<BlitzHost> ... </BlitzHost>`

If the current `oxidase` integration is not yet sufficient to replace those, then it is not done.

### C. Keep `dioxus-native-dom` dependency-clean

Do not add a direct dependency from `dioxus-native-dom` to `blitz-host`.

If additional support is needed, prefer:

1. narrowly-scoped API hooks
2. `oxidase`-side integration
3. separate glue owned above the DOM crate

But do not make `dioxus-native-dom` a `blitz-host` consumer.

---

## 4. The Tag / Dependency Story Must Be Honest

You must not claim a Git/tag dependency line that is not real.

That means:

### If you keep saying downstream users can depend on:

```toml
oxidase = { git = "https://github.com/techton7/oxidase.git", tag = "oxidase-vX.Y.Z", features = ["blitz-host"] }
```

then you must ensure the referenced tag actually contains the integration.

### Concretely

If `oxidase-v0.1.4` does **not** contain the finished integration, then:

1. do **not** keep pointing to `oxidase-v0.1.4` as if it does
2. either:
   - publish the next honest tag (for example `oxidase-v0.1.5` / `oxidase-macro-v0.1.5`) after validation
   - or explicitly state that the dependency line is not yet remotely consumable

Pick one and be truthful.

Do not leave the result in a half-published, half-local fantasy state.

---

## 5. Required Direction in Code

You must move the actual integration far enough into `oxidase` that the runner becomes a true consumer of that ergonomic boundary.

That may require:

1. `oxidase` optional dependency and feature wiring
2. `oxidase::launch` / `#[oxidase::main]` native bootstrap changes
3. a feature-gated wrapper/init mechanism inside `oxidase`
4. cleanup of runner dependencies and app code

But the final externally visible truth must be:

> the runner uses `oxidase` and the `blitz-host` feature, not raw `blitz_host` calls in app code.

---

## 6. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/Cargo.toml`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/Cargo.toml`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/src/launch.rs`
7. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/src/lib.rs`
8. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/src/prelude.rs`
9. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-macro/src/main_macro.rs`
10. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/Cargo.toml`
11. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/src/main.rs`
12. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/blitz/packages/dioxus-native-dom/`

---

## 7. Validation You Must Run

Run the smallest commands that prove the boundary is real.

At minimum:

1. `cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture`
2. `cargo check --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/Cargo.toml -p oxidase --features native,blitz-host`
3. `cargo build --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/Cargo.toml`
4. proof that the runner still supports attach / inspect / click / settle / changed-state verification
5. if you publish a new `oxidase` tag, verify the remote tags afterward

Do not call this complete based only on local compile or on stale previously-built binaries.

If markdown files are edited, validate them.

---

## 8. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. whether the runner still directly imports or calls `blitz_host`
2. whether ordinary `#[oxidase::main]` is now sufficient in the runner
3. whether the Git/tag dependency story is now actually true remotely
4. whether a new `oxidase` tag was required and, if so, what it is
5. what remains unresolved

---

## 9. Final Verdict Rule

You may report **Implemented and directionally integrated** only if:

1. the runner no longer directly imports or calls `blitz_host` in app code
2. the `oxidase` boundary is materially real rather than aspirational
3. the dependency/tag story is honest
4. `dioxus-native-dom` remains free of direct `blitz-host` dependency
5. the proven attach / inspect / click / settle flow still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> finish the actual move to the `oxidase` ecosystem boundary instead of merely saying it happened.
