# Worker Instruction: Implement `querySelector`-Style Selector Targeting in `blitz-host` (Still No Rhai)

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The direction is now fixed:

> **`querySelector` / selector-based targeting should be implemented in `blitz-host` now**

At the same time:

> **runtime scripting (Rhai / eval / run / REPL) is still deferred and must not be implemented in this pass**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Add selector-based targeting so that `blitz-host` users no longer need to depend only on ephemeral numeric `node_id`s for common workflows.

The point is:

> **selector ergonomics now, scripting later**

The selector work should meaningfully improve the current CLI/client surface without dragging in a scripting runtime.

---

## 2. Scope Boundary

### Must implement

1. selector-based targeting in the protocol / bridge
2. selector-based targeting in the client / CLI surface
3. live proof that selectors resolve and actions operate on the resolved nodes

### Must not implement

1. Rhai
2. `eval`
3. `run <file>`
4. REPL / shell
5. general-purpose scripting wrappers

This is a selector ergonomics pass, not a scripting pass.

---

## 3. Suggested Targeting Model

Use a small polymorphic targeting model, for example:

1. direct numeric node ID
2. CSS selector string

The exact type shape is up to you, but it should let a caller say either:

1. “act on node `4294967402`”
2. “act on `#submit-button`”

without inventing a full scripting layer.

Reasonable commands to support first:

1. `inspect`
2. `click`
3. `focus`
4. `set-value`
5. `capture`

You may decide whether keyboard/mouse actions should also accept selectors in the same pass if it stays coherent and does not sprawl.

---

## 4. Existing Engine Seam to Reuse

You should first inspect and then reuse the native selector/query support that already exists in the current engine/runtime stack where possible.

Do not re-implement a CSS selector engine from scratch.

The purpose of this pass is to expose that power through `blitz-host`, not to recreate Stylo/DOM selector machinery.

---

## 5. CLI / UX Goal

The ergonomic target is that users can write things like:

```bash
blitz-host inspect "#test-input"
blitz-host focus "#test-input"
blitz-host set-value "#test-input" "hello"
blitz-host mouse click "#submit-button"
blitz-host capture "#mouse-test-card" -o target/card.png
```

You do not need to implement every example above if one or two are enough to prove the model, but the direction should be that selector-based targeting is real and usable.

Keep the visible CLI surface simple.

---

## 6. Proof Expectations

The proof must show that selectors resolve on the live UI thread against the current live DOM/document, not against stale cached JSON.

At minimum, prove:

1. selector resolves to the expected live target
2. action executes against that target
3. the resulting state change is inspect-visible

Good proof cases:

1. `#test-input`
2. `#test-interaction-button`
3. `#mouse-test-card`

Do not stop at “selector parsed successfully.”

The proof is successful selection **plus** successful action/outcome.

---

## 7. What Not to Do

1. do not add scripting
2. do not implement a parallel selector engine if an existing seam is available
3. do not over-expand the action matrix in the same pass
4. do not make the new selector surface so magical that it becomes ambiguous or hard to debug

This should still feel like a deterministic control plane.

---

## 8. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/handoff.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/`
7. relevant Blitz / DOM selector APIs in the current engine stack

---

## 9. Validation You Must Run

Run the smallest commands that honestly prove selector targeting works.

At minimum:

1. protocol / transport tests for the new target representation
2. any focused selector-resolution tests available or needed
3. live native E2E proving selector-based targeting against a running host
4. confirmation that existing node-id-based flows still work

If markdown files are edited, validate them.

---

## 10. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what selector targeting surface was added
2. what engine/runtime selector seam it reuses
3. which commands now support selectors
4. what live proof was observed
5. that Rhai/runtime scripting remains deferred
6. what remains unresolved

---

## 11. Final Verdict Rule

You may report **Implemented and proven** only if:

1. selector-based targeting is real
2. it works against the live current DOM, not cached prior inspect data
3. at least one action/outcome flow is proven through selectors
4. runtime scripting is still clearly out of scope in this pass

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make `blitz-host` selector-ergonomic without turning it into a scripting platform.
