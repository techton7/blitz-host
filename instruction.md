# Worker Instruction: Fix and Finish the Core Mouse / Pointer / Wheel Lane Truthfully

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The previous pass moved the pointer/wheel lane in the right direction, but it is **not complete yet** because the live native E2E proof is currently failing.

This pass must first restore a truthful green baseline and then finish the slice.

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Finish the current core mouse / pointer / wheel slice **for real**.

That means:

1. identify and fix the current hover-proof mismatch
2. make the implementation and the proof expectations agree
3. rerun the full `blitz-host` suite honestly
4. only then report completion

The immediate problem is not “which future pointer features to add.”

The immediate problem is:

> the current live pointer proof is failing, so the result text is ahead of reality.

---

## 2. Known Current Failure

At the time of this instruction, the following command does **not** pass cleanly:

```bash
cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture
```

The concrete observed failure is in the live native E2E hover proof:

1. target node (`#mouse-test-card`) had one ID
2. `InspectResponse.hover_node_id` returned a different node ID
3. the test panicked because its assertion expected the hover target to be the card itself or a narrowly-defined child

You must treat this as a real failing state.

Do not report success until it is fixed and the full suite passes.

---

## 3. The Real Design Question You Must Resolve

You must make the implementation and the proof model agree on what “hover target” means.

Possible outcomes include:

1. **the implementation is right and the test expectation is wrong**
   - for example, the true hover hit target is a deeper descendant than the test allowed
2. **the implementation is publishing the wrong hover identity**
   - for example, the inspect surface should be reporting a different node identity or additional context
3. **the inspect surface needs a clearer contract**
   - for example, some distinction between directly hit node vs. logical/semantic hover container

Pick the correct one from source/runtime evidence and align the code/tests/result accordingly.

---

## 4. Scope of This Pass

### Must do

1. repair the current failing hover proof
2. ensure the pointer/wheel surface is internally consistent
3. rerun the tests and live proof honestly
4. correct any overclaim in `result.md`

### May do

If fixing the hover mismatch reveals small adjacent consistency issues caused by the same change, fix those too.

### Must not do

1. do not jump to another new feature slice
2. do not weaken the proof by making the test meaningless
3. do not hide the issue with vague wording

This is a “make the current mouse lane true” pass.

---

## 5. Required Truthfulness Standard

You must align:

1. protocol meaning
2. inspection output
3. bridge/runtime behavior
4. live test expectations
5. result narrative

If the implementation returns one identity while the proof assumes a different identity, you must resolve that mismatch explicitly.

Do not simply declare the current behavior “good enough” without deciding what the hover contract actually is.

---

## 6. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/src/inspect.rs`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/tests/live_inspect.rs`
5. any pointer dispatch code in `dioxus-native-dom` or related native event plumbing
6. any example/harness UI elements used as pointer proof targets

---

## 7. Validation You Must Run

Run the same command that is currently known to fail and make it pass honestly:

```bash
cargo test --manifest-path /Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/Cargo.toml -- --nocapture
```

Also rerun any additional targeted commands needed to justify the final hover/pointer contract.

Do not call this complete until the full `blitz-host` suite is green again.

If markdown files are edited, validate them.

---

## 8. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what the hover mismatch actually was
2. whether the implementation or the proof expectation changed
3. what the final hover contract is
4. what validation now passes
5. what remains deferred

---

## 9. Final Verdict Rule

You may report **Implemented and proven** only if:

1. the full `blitz-host` suite is green again
2. the hover/pointer contract is explicit and internally consistent
3. the result text no longer overclaims
4. the mouse / pointer / wheel lane proof is real

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make the current pointer/wheel slice actually true in code, tests, and explanation before moving on.
