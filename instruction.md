# Worker Instruction: Build the `blitz-host` `act + settle` Vertical Slice

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The inspect-only vertical slice is already implemented and runtime-proven.

The next slice is now:

> **send a real action into a live running Blitz window, wait for the UI to settle, and prove the changed state through a follow-up inspect**

This is the entire goal of this pass.

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Extend the existing `blitz-host` inspect stack so that it can:

1. send at least one real action to the running host
2. execute that action on the correct UI thread
3. wait until synchronous/reactive work has settled enough to read stable state
4. re-inspect and prove the state change

The milestone question is:

> can an agent click a real live UI element in the running Blitz host and then observe the resulting state change through inspect?

That is the next proof target.

---

## 2. Scope Boundary

This pass is **not** a full action platform.

### Required in this slice

1. a minimal typed action request surface
2. UI-thread-safe action dispatch
3. a concrete settle strategy after mutation
4. runner-side observable state change target
5. end-to-end proof: inspect → act → settle → inspect

### Explicitly defer for later

Do **not** turn this into a broad action suite unless it is almost free:

1. full keyboard/input matrix
2. hover/pointer drag
3. scroll control
4. capture/image proof
5. lifecycle commands
6. multi-client behavior

Keep the slice narrow and finish the real proof.

---

## 3. Current Facts You Should Start From

Unless reinspection disproves them:

1. `blitz-host` already has:
   - `blitz-host-protocol`
   - `blitz-host-transport`
   - `blitz-host-bridge`
   - `oxidase-native-runner --debug-control` integration
2. inspect is already runtime-proven against a real running native Blitz window
3. the runner UI already contains:
   - a real button with `id="test-interaction-button"`
4. the best next proof is to make that button trigger an inspect-visible state change

This pass should build directly on the working inspect path, not redesign it.

---

## 4. Required Action Surface

Add the **smallest useful typed action surface**.

Recommended minimum:

1. `Click { node_id: u64 }`

If a second action is nearly free, acceptable next candidate:

2. `Focus { node_id: u64 }`

But the pass can be considered successful with **click only**, if the proof is solid.

Do not expand beyond that unless it is low-cost and does not risk the schedule.

---

## 5. Required Settle Behavior

This is the most important part of the slice.

After a mutation-producing action, the system must not immediately pretend the UI is stable.

You must add a concrete settle strategy sufficient for this pass.

That can be some combination of:

1. draining queued bridge work
2. waiting for one or more frame turns
3. polling until an expected inspect-visible condition stabilizes
4. a bounded settle loop with explicit failure if stability is not reached

### Important rule

The proof must demonstrate that the post-action inspect is reading **changed settled state**, not just racing the mutation.

Do not solve this with a vague sleep-only hack if a more host-aware bounded strategy is available.

---

## 6. Runner Proof Target

Use the existing runner UI as the proof harness.

Required change:

1. make the button with `id="test-interaction-button"` produce an inspect-visible state change

Examples of acceptable proof targets:

1. increment a visible counter
2. toggle a visible status text
3. change an element label or badge

The resulting state change must be something the inspect API can actually observe in the semantic tree.

### Preferred proof shape

1. inspect tree
2. locate button node by `dom_id`
3. send `Click`
4. wait for settle
5. inspect again
6. assert the visible state changed as expected

---

## 7. Protocol / Bridge / Runner Work Required

### `blitz-host-protocol`

Extend the protocol with the smallest action vocabulary needed for this slice.

### `blitz-host-transport`

Extend the client/server path so action requests can be sent and acknowledged.

### `blitz-host-bridge`

Implement action dispatch on the UI thread.

This must target the real live document/window, not a synthetic mock.

### `oxidase-native-runner`

Add the inspect-visible proof state change and ensure the debug-control lane can service action requests safely while the runner is live.

---

## 8. Critical Design Constraints

### Must do

1. preserve the already-working inspect path
2. keep action execution on the correct UI thread
3. produce a real inspect-visible state change
4. prove post-action settled inspection
5. keep the scope small

### Must not do

1. do not redesign the whole protocol for future actions
2. do not expand into capture/lifecycle unless almost free
3. do not claim settle correctness without a real proof
4. do not replace a real end-to-end proof with mock-only tests

---

## 9. Files to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/src/lib.rs`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/src/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/src/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/src/main.rs`
7. current live proof harness/tests for inspect

---

## 10. Validation You Must Run

Run the smallest commands that prove this slice honestly.

At minimum:

1. `cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture`
2. whatever targeted runner command is needed for live proof against `--debug-control`
3. actual end-to-end proof of:
   - inspect
   - click
   - settle
   - re-inspect with changed state

Do not call this complete based only on static compilation or unit tests.

If markdown files are edited, validate them.

---

## 11. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what action vocabulary was added
2. how action dispatch works on the UI thread
3. what settle strategy was chosen
4. what runner state change was used as the proof target
5. the exact end-to-end proof observed
6. what remains deferred to later slices

---

## 12. Final Verdict Rule

You may report **Implemented and runtime-proven** only if:

1. a real action request can be sent to the live runner
2. the runner executes it on the correct UI thread
3. the system waits for settled state well enough to make the proof trustworthy
4. a follow-up inspect confirms the expected changed state

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The outcome of this task is:

> a real `inspect → click → settle → inspect changed state` proof on the live internal Blitz runner.
