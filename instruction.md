# Worker Instruction: Add a Minimal `capture` / Visual Proof Lane to `blitz-host`

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current stack already proves:

1. attach
2. inspect
3. click
4. focus
5. set-value
6. settle
7. deterministic process targeting

The next major `blitz-host`-specific value is now:

> **visual proof beyond semantic DOM — a minimal capture path**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Implement the smallest useful visual capture lane for `blitz-host`.

This should let an external client prove not only:

1. what the semantic tree says
2. what the input state says

but also:

3. what the rendered output actually looks like

The core question is:

> can `blitz-host` capture a real visual result from the running native host in a way that is useful for debugging and proof?

---

## 2. Scope Boundary

Keep this slice intentionally small.

### Must aim for

1. one-shot capture
2. a useful output format
3. proof against a live native host

### Explicitly defer

Do **not** expand into:

1. streaming/video
2. diff engines
3. per-node region capture unless almost free
4. large visual tooling suite
5. keyboard matrix expansion in this pass

This pass is about the first real visual proof seam.

---

## 3. Preferred Minimal Outcome

The preferred first capture target is:

1. capture the current rendered window/document view
2. return it in a practical machine-usable format
3. prove it against a running native host

Reasonable output forms include:

1. PNG bytes
2. base64-encoded image payload
3. a file output path written by the CLI if that is the cleanest practical surface

Pick the smallest honest form that fits the current renderer/runtime seams.

---

## 4. Architectural Guidance

### A. Treat capture as a `blitz-host` concern

This is exactly the kind of feature that is more `blitz-host`-specific than `blitz`-generic:

1. transporting proof artifacts
2. turning renderer state into debug-observable output
3. exposing that through CLI/client APIs

So this is a better next expansion than a giant keyboard behavior matrix.

### B. Stay honest about what seam you use

If the current stack only allows:

1. window-level full capture
2. or host-level framebuffer capture

then implement that and say so.

Do not imply subtree or exact per-node screenshots unless you really have them.

### C. If blocked, produce a real blocker

If direct capture is not honestly implementable with the current Blitz/Vello/runtime seams, do not fake it.

Instead:

1. inspect the available renderer/readback hooks
2. attempt the smallest honest implementation
3. if blocked, report the concrete seam missing

This is acceptable if the evidence is solid.

---

## 5. Suggested Implementation Shape

If feasible, the likely path is:

1. protocol:
   - add `CaptureRequest`
   - add `CaptureResponse`
2. bridge / host:
   - invoke the smallest available native capture/readback path
3. transport/client:
   - expose `capture(...)`
4. CLI:
   - add `blitz-host capture`
   - optionally write to a file or stdout/json depending on the cleanest UX

Keep the user-facing interface small and practical.

---

## 6. Proof Targets

Use the existing established lanes:

### A. Canonical cross-host example

Use `cross_host` as the first conceptual proof target when appropriate.

### B. Native proof harness

Use `oxidase-native-runner` as the native proof harness if it is the easiest place to verify capture correctness and stability.

If one target is clearly more practical for the first capture proof, use it first and explain why.

---

## 7. What Not to Do

1. do not overpromise subtree capture if you only have full-window capture
2. do not treat semantic inspection and capture as the same thing
3. do not skip proof and stop at a compile-only transport shape
4. do not turn this into a giant media/export subsystem

Keep it to:

> first honest visual proof

---

## 8. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/`
6. relevant renderer / Vello / Blitz host seams that could support capture or readback

---

## 9. Validation You Must Run

Run the smallest commands that honestly prove the capture lane.

At minimum:

1. protocol / transport tests for the new capture surface
2. any renderer-side or bridge-side focused checks needed for capture
3. live native proof against a running host
4. confirmation that existing attach / inspect / click / focus / set-value flows still work

If markdown files are edited, validate them.

---

## 10. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what capture surface was added
2. what exactly is captured (full window, document, etc.)
3. what format is returned or written
4. what live proof was observed
5. what remains deferred
6. if blocked, the exact renderer/runtime seam that blocked honest capture

---

## 11. Final Verdict Rule

You may report **Implemented and proven** only if:

1. a real capture surface exists
2. it works against a live native host
3. its scope is stated honestly
4. the existing control-plane proof path still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> give `blitz-host` its first real visual proof capability beyond semantic DOM inspection.
