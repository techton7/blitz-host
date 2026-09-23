# Worker Instruction: Expand `blitz-host` to a Bounded Core Keyboard Lane

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current `blitz-host` stack already proves:

1. attach
2. inspect
3. click
4. focus
5. set-value
6. settle
7. capture
8. process targeting

The next slice should now expand beyond the initial two-key idea into a **bounded core keyboard lane**.

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Add the smallest *useful* core keyboard surface for real workflow automation.

This pass should cover:

### A. Focus / activation keys

1. `Tab`
2. `Shift+Tab`
3. `Enter`
4. `Space`
5. `Escape`

### B. Text editing essentials

1. `Backspace`
2. `Delete`

### C. Navigation essentials

1. `ArrowLeft`
2. `ArrowRight`
3. `ArrowUp`
4. `ArrowDown`

### D. One modifier sentinel

1. **`Ctrl/Cmd + A`** as the explicit modifier-path proof case

The purpose is not “all keyboard behavior forever.”

The purpose is:

> prove that the real keyboard injection path is broad enough to drive common app workflows, including one meaningful modifier combination.

---

## 2. Responsibility Boundary

Keep the responsibility split honest:

1. **Blitz / Dioxus Native own keyboard semantics**
   - focus traversal behavior
   - submit behavior
   - selection behavior
   - text editing behavior
2. **`blitz-host` owns**
   - key injection surface
   - transport/protocol expression
   - settle / inspect observation
   - black-box proof that the real runtime behavior happened

Do **not** re-implement browser/editor semantics inside `blitz-host`.

This is a control-plane proof task, not a new text engine.

---

## 3. Scope Boundary

### Must implement

1. a typed keyboard action surface sufficient for the core key set above
2. modifier support sufficient for `Ctrl/Cmd + A`
3. black-box proof targets that demonstrate real workflow effects

### Explicitly defer

Do **not** expand into:

1. full shortcut matrix
2. IME / composition
3. clipboard shortcuts
4. platform-specific accelerator universe
5. giant editor behavior suite

Keep it to the bounded core set.

---

## 4. Preferred Proof Targets

You must prove keyboard behavior through inspect-visible workflow changes.

### A. Focus traversal proof

Use `Tab` / `Shift+Tab` to prove:

1. focus moves between expected elements
2. inspect reflects the new focused node

### B. Submit / activation proof

Use `Enter` and/or `Space` to prove:

1. a focused button or submit target activates
2. a visible status / state change occurs

### C. Editing proof

Use `Backspace` / `Delete` / arrow keys as appropriate to prove:

1. text editing state changes
2. cursor/navigation-sensitive behavior is actually going through the runtime

### D. Modifier sentinel proof

Use **`Ctrl/Cmd + A`** to prove modifier-path integrity.

The preferred black-box scenario is:

1. set an input to a known longer string
2. focus the input
3. inject `Ctrl/Cmd + A`
4. replace or overwrite text afterward
5. inspect and prove the replacement happened as expected

This gives you real coverage of:

1. modifier serialization
2. platform-aware mapping
3. selection pipeline
4. follow-up editing behavior

---

## 5. Platform Mapping Rule

For the modifier sentinel, be honest about platform reality:

1. macOS should use `Cmd+A`
2. Windows/Linux should use `Ctrl+A`

If you introduce a higher-level “select all” semantic helper internally, that is acceptable.

If you keep a raw key+modifier surface, that is also acceptable.

But the proof must explicitly show that modifier-aware behavior works on the current native target.

---

## 6. Preferred Test Route

Use the same honest three-stage route:

### Stage 1 — protocol / transport unit route

Validate:

1. serialization of the new keyboard action surface
2. transport/client request and response handling
3. no regression in existing control-plane features

### Stage 2 — headless semantics route

Use the cheapest honest route available (for example a headless Dioxus/Blitz harness) to verify:

1. `Tab` / `Shift+Tab`
2. `Enter` / `Space`
3. editing keys
4. `Ctrl/Cmd + A`

This is where you cheaply falsify bad key injection or event-model assumptions.

### Stage 3 — live native E2E route

Final proof should use:

1. canonical `cross_host` example as the first public-consumer proof target
2. `oxidase-native-runner` as the secondary/native-proof harness target if useful

The first-class story should still be the canonical consumer example where practical.

---

## 7. What Not to Do

1. do not expand to full keyboard universe
2. do not claim coverage of all modifiers just because `Ctrl/Cmd + A` works
3. do not stop at “request returned success”
4. do not let hidden internal state replace inspect-visible proof

This pass should remain:

> core keyboard workflow support, not total keyboard completeness.

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
8. any headless harness route suitable for cheap keyboard semantics validation

---

## 9. Validation You Must Run

Run the smallest commands that honestly prove the slice.

At minimum:

1. protocol / transport tests for the new keyboard surface
2. a headless semantics proof for the bounded core key set
3. live native E2E against `cross_host`
4. continued proof or regression checks against `oxidase-native-runner`
5. confirmation that attach / inspect / click / focus / set-value / capture still work after the change

Do not call this complete if only the low-level request succeeds but the workflow effect is unproven.

If markdown files are edited, validate them.

---

## 10. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what keyboard action surface was added
2. how focus traversal was proven
3. how submit/activation was proven
4. how editing/navigation was proven
5. how the modifier sentinel (`Ctrl/Cmd + A`) was proven
6. what headless semantics route was used
7. what live E2E proof was observed
8. what remains deferred

---

## 11. Final Verdict Rule

You may report **Implemented and proven** only if:

1. the bounded core key set is real
2. `Ctrl/Cmd + A` proves modifier-path integrity
3. the resulting workflow changes are inspect-visible after settle
4. the proof remains black-box and honest
5. existing control-plane functionality still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> turn `blitz-host` from click/input primitives into a bounded but genuinely useful keyboard workflow control surface.
