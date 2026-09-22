# Worker Instruction: Turn the Proven `blitz-host` Spike into a Usable Facade

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current `blitz-host` spike has already proven the hard technical core:

1. attach to a live Blitz window
2. inspect semantic state
3. dispatch click
4. settle
5. re-inspect changed state

So the next task is **not** “add another low-level trick.”

The next task is:

> **make `blitz-host` structurally usable instead of leaving the current proof scattered across low-level crates and manual runner wiring**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Take the current working spike and refactor it into a cleaner, more usable shape centered on a **single top-level `blitz-host` facade crate**.

The main outcomes should be:

1. a top-level `blitz-host` crate exists
2. it presents the usable external surface
3. the CLI binary belongs to that surface
4. `oxidase-native-runner` no longer needs to wire the low-level `protocol / transport / bridge` crates directly in an ugly ad-hoc way
5. the current proven behavior still works after the cleanup

This is a structural cleanup and packaging pass built on top of the already-proven core.

---

## 2. The Problem You Are Solving

The current spike works, but it has two real structural problems:

### Problem A — the usable surface is fragmented

Right now the implementation is spread across:

1. `blitz-host-protocol`
2. `blitz-host-transport`
3. `blitz-host-bridge`

That is a good internal split, but it is not a good top-level user-facing shape by itself.

### Problem B — the runner integration is too low-level

Right now `oxidase-native-runner` knows too much about:

1. low-level bridge types
2. transport startup
3. static storage / plumbing
4. manual per-frame servicing details

The proof is valid, but the integration shape is too exposed and too messy.

The job now is to clean that up **without pretending the world is magically zero-wiring for every app**.

---

## 3. Current Facts You Should Start From

Unless reinspection disproves them:

1. `blitz-host` already works technically for:
   - inspect
   - click
   - settle
   - changed-state verification
2. the current proof is real and must be preserved
3. the internal 3-way split is still valuable:
   - `blitz-host-protocol`
   - `blitz-host-transport`
   - `blitz-host-bridge`
4. what is missing is a **clean facade and cleaner host-side integration**
5. you must not turn this into fiction about official upstream Dioxus or universal automatic injection

---

## 4. Required Structural Direction

### A. Create a top-level facade crate

Add a top-level crate:

```text
util/blitz-host/crates/blitz-host
```

Its job is to provide the main usable surface.

At minimum it should:

1. re-export the public parts of:
   - protocol
   - transport/client
   - bridge
2. own the canonical CLI binary surface
3. provide a higher-level integration surface for internal hosts/runners

### B. Keep the internal split

Do **not** collapse everything into one file or destroy the internal architecture.

The correct model is:

1. internal implementation remains split
2. external usability is unified behind `blitz-host`

That is the distinction you are trying to achieve.

---

## 5. Required Integration Cleanup

You must reduce how much raw low-level `blitz-host-*` knowledge leaks into `oxidase-native-runner`.

The runner should still be the proof harness, but the integration should be cleaner.

That means introducing a **higher-level integration helper** at the `blitz-host` facade layer.

This helper does not need to be perfect or universal yet, but it should absorb obvious boilerplate such as:

1. starting the debug-control server
2. holding the bridge/runtime state
3. exposing a cleaner servicing API to the host

### Explicit runner recovery requirement

The worker previously got the proof by hardcoding too much low-level `blitz-host` plumbing directly into `oxidase-native-runner`.

This cleanup pass must explicitly address that.

At minimum, the refactor should aim to:

1. stop making `oxidase-native-runner` directly orchestrate multiple low-level `blitz-host-*` crates in an ad-hoc way
2. move that knowledge behind the new top-level `blitz-host` facade
3. give the runner a much smaller integration surface

### Preferred integration shape

The preferred outcome is:

> in debug-control mode, the runner can enable `blitz-host` through a single high-level facade-level init/integration entrypoint (or the smallest honest equivalent), rather than open-coding server startup, static state, document-handle plumbing, and low-level bridge servicing all over the runner.

Do not fake “one line” if the runtime truth still requires two or three explicit touchpoints.

But do actively try to compress the current ugly wiring into the smallest clean integration surface that is technically honest.

### Important constraint

Do not claim this means “all Dioxus apps automatically support blitz-host.”

This is still an internal opt-in integration helper, not magic upstream runtime support.

Be honest about that in code and documentation.

---

## 6. CLI / Surface Direction

The canonical user-facing CLI should belong to the top-level `blitz-host` surface, not feel like an accidental byproduct of `blitz-host-transport`.

Preserve the cleaned command direction that already emerged:

1. `blitz-host inspect`
2. `blitz-host click <NODE_ID>`
3. per-subcommand `--help`

Do not re-expand the CLI surface unnecessarily in this pass.

This pass is about structural cleanup, not new command proliferation.

---

## 7. What You Must Preserve

Do not lose the currently proven vertical slice.

After your refactor, these must still work:

1. live runner attach
2. inspect
3. click
4. settle
5. changed-state proof

If the cleanup makes the code prettier but breaks the proof, it is not acceptable.

---

## 8. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/`

And then add the new facade crate:

7. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/`

---

## 9. Validation You Must Run

Run the smallest commands that prove the cleanup preserved reality.

At minimum:

1. `cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture`
2. any additional crate-level checks needed after adding the facade crate
3. proof that `oxidase-native-runner --debug-control` still supports:
   - inspect
   - click
   - settle
   - changed-state re-inspection

Do not call this complete based only on compile success.

If markdown files are edited, validate them.

---

## 10. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what facade crate was added
2. what it re-exports
3. where the canonical CLI surface now lives
4. how runner integration became cleaner
5. how much of the old hardcoded low-level runner wiring was removed or hidden behind the facade/integration helper
6. what low-level internals still remain split
7. what proof still passed after the cleanup
8. what is still **not** solved (for example: universal upstream integration)

---

## 11. Final Verdict Rule

You may report **Implemented and runtime-proven** only if:

1. the top-level `blitz-host` facade crate exists
2. the CLI surface is routed through that facade cleanly enough
3. runner integration is meaningfully cleaner than before, and the old ugly hardcoded low-level spike wiring in `oxidase-native-runner` is materially reduced or hidden behind facade-level init/helper code
4. the previously proven act/settle proof still works after the refactor
5. the result report stays honest about current limits

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The next step is not new capability for capability’s sake.

It is:

> turn the proven low-level spike into a cleaner and more usable internal product shape.
