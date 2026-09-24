# Worker Instruction: Add the First `subtree / node-level capture` Lane via Crop-Based Capture

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current stack already proves:

1. attach
2. inspect
3. click / focus / set-value
4. bounded keyboard lane
5. core mouse / pointer / wheel lane
6. full-window capture
7. process targeting

The next visual refinement is now fixed:

> **add the first `subtree / node-level capture` lane**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Implement the first useful `node` / `subtree` visual capture lane.

The intended first version should be:

1. inspect-selected node
2. capture the full rendered surface
3. crop the image to the node’s visual bounds
4. return or write the cropped image as the proof artifact

This is the right first step because it gives strong local visual proof without requiring a larger renderer redesign.

---

## 2. Preferred Implementation Strategy

Use the current working seams.

### Preferred v1 approach

> **full capture + node-bounds crop**

That means:

1. reuse the current full-window/document capture pipeline
2. resolve node bounds from the existing inspect/layout data
3. crop the rendered image to the selected node rect
4. emit that crop as the result

This is preferred over attempting a full “true subtree render” in the first pass.

### Important note

Do **not** add new external repo dependencies just to do this first slice if the current code already has what is needed.

You may refer to prior reference work for ideas, but the implementation should stand on the current local seams unless a hard blocker appears.

---

## 3. Scope Boundary

### Must implement

1. requesting capture for a specific node/subtree
2. bounds-based crop of the current rendered surface
3. live proof that the resulting artifact corresponds to the selected node

### Explicitly defer

Do **not** expand into:

1. full subtree-aware renderer specialization
2. visual diff engines
3. video / streaming capture
4. arbitrary region selection unrelated to inspect-selected nodes

Keep it to:

> inspect-selected node/subtree crop

---

## 4. Suggested Surface

The exact type shape is up to you, but a reasonable direction is:

1. extend `CaptureRequest` with `node_id: Option<u64>`
2. when `node_id` is `None`, preserve current full-window capture behavior
3. when `node_id` is `Some(id)`, capture full surface and crop to the node’s bounds

If a node’s bounds are missing or invalid, report that honestly instead of silently pretending capture succeeded.

---

## 5. Required Proof Targets

You must prove that the cropped image meaningfully corresponds to the selected node.

Good proof targets include:

1. `#test-interaction-button`
2. `#test-input`
3. `#mouse-test-card`
4. another visually distinct element already used in the canonical example / harness

The proof should demonstrate:

1. inspect finds the node and its bounds
2. capture with `node_id` produces a smaller crop than the full window where appropriate
3. the crop dimensions match or correspond to the node bounds
4. the image artifact is valid and non-empty

If possible, use an element with distinctive styling so the crop is clearly meaningful.

---

## 6. Honesty Constraints

You must be explicit about what this first capture lane really is.

If it is:

> **full-scene render followed by crop**

say that plainly.

Do **not** describe it as if the renderer is doing a native node-only render pass unless that is actually true.

This matters because the implementation strategy is acceptable — but only if reported honestly.

---

## 7. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
6. current capture code and current inspect/bounds code
7. canonical `cross_host` example and/or `oxidase-native-runner` for proof targets

---

## 8. Validation You Must Run

Run the smallest commands that honestly prove the slice.

At minimum:

1. the relevant `blitz-host` tests after changing capture surfaces
2. at least one live native proof against a running host
3. verification that the cropped artifact is valid and non-empty
4. confirmation that the existing full-window capture still works

If markdown files are edited, validate them.

---

## 9. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what request surface changed
2. whether the implementation is crop-based or true subtree render
3. how node bounds are resolved
4. what live proof was observed
5. what still remains deferred

---

## 10. Final Verdict Rule

You may report **Implemented and proven** only if:

1. node-level capture really exists
2. it is proven against a live native host
3. the implementation scope is described honestly
4. full-window capture and existing control-plane features still work

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> connect inspect-selected nodes to local visual proof through the smallest honest node/subtree capture implementation.
