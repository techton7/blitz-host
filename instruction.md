# Worker Instruction: Extend `blitz-host` with `Focus` + `SetValue` and Prove It Through a Three-Stage Test Route

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current `blitz-host` stack has already proven:

1. attach
2. inspect
3. click
4. settle
5. deterministic process targeting (`list` + `--pid`)

The next slice is now:

> **move from “can click a button” to “can drive a real input workflow”**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Add the next practical input/control surface to `blitz-host`:

1. `Focus`
2. `SetValue`

and prove that they work through the full stack.

The proof target is:

> a live host can be attached, the right input can be focused, a value can be injected, the UI can settle, and the changed state can be observed through inspect.

---

## 2. Scope Boundary

This pass is intentionally **not** the full input matrix.

### Must implement

1. focus action
2. set-value action
3. whatever bridge/runtime support is needed for those actions
4. a real inspect-visible proof target for the new workflow

### Explicitly defer

Do **not** expand further unless nearly free:

1. generalized keyboard sequence matrix
2. drag / hover / pointer move
3. scroll
4. capture
5. broad diagnostics

Keep the slice narrow and finish the real workflow proof.

---

## 3. Current Facts You Should Start From

Unless reinspection disproves them:

1. process-level attach/selection is already working
2. click + settle proof is already working
3. `cross_host` is now the canonical cross-host example
4. `oxidase-native-runner` remains the native-specific proof harness
5. the next value jump is real text/input workflow control, not more selector bikeshedding

That is the next problem to solve.

---

## 4. Required Action Surface

Extend the current action vocabulary with the minimum useful next actions.

### A. `Focus`

This should allow the client to target a focusable node and make it the active element.

### B. `SetValue`

This should allow the client to set text/input value on a target element through the real event/runtime path, not via fake test-only mutation shortcuts.

### Important rule

The implementation must preserve the philosophy already established:

> use the real native / Dioxus event plumbing where possible, not a separate fake state channel.

---

## 5. Required Proof Target

You need a real, inspect-visible workflow target.

Add or adapt the canonical app/harness UI so that the new workflow can be proven clearly.

Reasonable shape:

1. input field with stable ID (for example `id="test-input"`)
2. inspect-visible derived state (for example `Typed: hello`)
3. optional secondary control like a submit button if needed

The proof must demonstrate:

1. locate the input via inspect
2. focus it
3. set its value
4. settle / wait as needed
5. re-inspect and confirm the updated visible state

Do not stop at “action call returned success.”

The proof is the changed state.

---

## 6. Required Three-Stage Test Route

This pass must follow the agreed testing route.

### Stage 1 — protocol / transport unit route

Validate:

1. serde roundtrip for new action types
2. transport/client response handling
3. no regression in attach / selector model

### Stage 2 — headless semantics route

Use the cheapest honest route available (for example `blitz-test-harness` or an equivalent minimal document-level harness) to verify the event semantics themselves:

1. focus is really applied
2. set-value follows the real path you intend
3. the expected DOM/semantic state changes happen after pump/tick/settle

This stage exists to catch event/model issues before the expensive live native run.

### Stage 3 — live native E2E route

Final proof must happen against live running native targets.

#### Preferred first proof target

1. canonical `cross_host` example

#### Secondary / harness proof target

2. `oxidase-native-runner`

The point is:

> public consumer story first, harness story second.

---

## 7. What Not to Do

1. do not jump straight to keyboard matrix complexity
2. do not skip the headless semantics route if it can cheaply falsify wrong event plumbing
3. do not replace inspect-visible proof with “action returned success”
4. do not turn this into a generic form automation platform in one pass

This slice is:

> focus + set-value + real proof

no more.

---

## 8. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/examples/cross_host/`
7. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/`
8. any available headless harness/utilities that can cheaply validate focus/value semantics

---

## 9. Validation You Must Run

Run the smallest commands that honestly prove the slice.

At minimum:

1. protocol / transport tests for the new action types
2. a headless semantics proof route (if available and honest)
3. live native E2E against `cross_host`
4. continued or secondary proof against `oxidase-native-runner`
5. confirmation that attach / inspect / click / settle still work after the change

Do not call this complete if only the unit tests pass.

If markdown files are edited, validate them.

---

## 10. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what new actions were added
2. how focus/value semantics are implemented
3. what headless semantics route was used (or why it was unavailable)
4. how the live E2E proof was run against `cross_host`
5. what role `oxidase-native-runner` played in the proof
6. what remains deferred

---

## 11. Final Verdict Rule

You may report **Implemented and proven** only if:

1. `Focus` is real
2. `SetValue` is real
3. changed state is proven through inspect after settle
4. the three-stage test route is followed honestly enough to justify the claim
5. existing attach / inspect / click / settle functionality still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make `blitz-host` capable of driving real input workflows, not just button clicks.
