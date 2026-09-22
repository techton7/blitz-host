# Worker Instruction: Add a Canonical Cross-Host `oxidase` Example and Keep the Native Proof Harness

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The `blitz-host` → `oxidase` boundary move is now established.

The next architectural step is also fixed:

> **`oxidase` should have a single canonical example that demonstrates the same app code path working on both Web and Native, while `oxidase-native-runner` remains as the native-specific proof harness.**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Deliver the correct split between:

1. **canonical cross-host consumer proof**
2. **native-specific proof harness**

Specifically:

### A. Add or promote one `oxidase` example

There should be a single canonical `oxidase` example that represents the actual product value proposition:

> the same app code path works on both Web and Native through `oxidase`

### B. Keep `oxidase-native-runner`

Do **not** remove `oxidase-native-runner` in this pass.

It should remain the place for:

1. auto-close proof mode
2. native-specific debug-control proof
3. deterministic attach / inspect / click / settle verification
4. proof-oriented logging and harness behavior

The example and the runner serve different roles and should be treated that way.

### C. Make the runner reproducible, not path-fragile

If `oxidase-native-runner` is going to remain a durable proof harness, its canonical dependency story should prefer fixed refs:

1. GitHub repository dependencies pinned by tag
2. published versions where appropriate

Local path overrides may still exist for active monorepo development convenience, but they must not be the only truth for the long-lived harness story if a more reproducible tagged path is intended.

---

## 2. The Intended Split

### Canonical `oxidase` example

This should prove the public cross-host story:

1. same UI code
2. Web support
3. Native support
4. `oxidase` ergonomics
5. when relevant, `blitz-host`/`--debug-control` attach path on Native

### `oxidase-native-runner`

This should remain the internal harness for native-specific proof:

1. proof mode
2. auto-close mode
3. attach/click/settle assertions
4. native host/runtime-specific validation

Do not confuse the two.

---

## 3. Current Facts You Should Start From

Unless reinspection disproves them:

1. the `blitz-host` feature integration now lives at the `oxidase` boundary
2. `oxidase-native-runner` no longer directly imports `blitz_host` in app code
3. the native proof harness still exists and still matters
4. what is missing now is the **canonical single example** that demonstrates the actual cross-host consumer story

That is the problem this pass should solve.

---

## 4. Required Example Direction

Create or promote one `oxidase` example under the `oxidase` crate that is the canonical cross-host sample.

It should be designed so that:

1. the same app/component code is meaningful on Web
2. the same app/component code is meaningful on Native
3. the example demonstrates the core `oxidase` value, not random unrelated UI

### Preferred example characteristics

The example should ideally show:

1. `#[oxidase::main]`
2. `use_frame`
3. `next_frame`
4. a small interactive state mutation that is visible in both Web and Native
5. if native debug-control proof is exercised against it, a stable inspect/click target

It does not need to become a huge showcase.

Keep it small and representative.

---

## 5. Relationship Between Example and Runner

The example and the runner should not drift into unrelated apps if avoidable.

If helpful, extract shared app/UI logic so that:

1. the example is the canonical consumer story
2. the runner reuses the same or very similar UI for proof-harness purposes

This is preferred if it keeps maintenance low and avoids divergence.

But do not force a bad abstraction if it becomes messy.

The key requirement is conceptual alignment:

> the runner should prove the same consumer story the example represents, while still adding native-specific proof machinery.

---

## 6. What Not to Do

1. do not delete `oxidase-native-runner`
2. do not let the runner remain the *only* place the cross-host value is shown
3. do not add multiple competing examples if one canonical example is enough
4. do not turn this into a large demo gallery

This pass is about clarifying the architecture, not multiplying surfaces.

---

## 7. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/examples/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-native-runner/`
6. the runner/example dependency wiring (`Cargo.toml`, feature flags, git tag pins, patch overrides)

Reinspect enough surrounding code before deciding whether to add a new example or refactor an existing one into the canonical example.

---

## 8. Validation You Must Run

Run the smallest commands that prove the example/runner split is real.

At minimum:

1. relevant `cargo check` / `cargo test` for `util/blitz-host`
2. compile validation for the chosen `oxidase` example
3. native validation for the example if you wire it that far
4. continued proof that `oxidase-native-runner` still works as the native-specific harness
5. evidence that the runner’s intended long-lived proof path is pinned to stable refs/tags rather than being silently dependent on local path state, if you change that wiring in this pass

If the example is intended to be the actual cross-host canonical sample, validate both:

1. Web compilation/path
2. Native compilation/path

Do not call this complete if only one host path works.

If markdown files are edited, validate them.

---

## 9. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what canonical `oxidase` example now represents the cross-host story
2. whether shared example/runner app logic was extracted
3. what role remains for `oxidase-native-runner`
4. whether the runner dependency story was moved to stable GitHub tags / versions or remains path-bound, and why
5. how Web and Native were each validated
6. what still remains unresolved

---

## 10. Final Verdict Rule

You may report **Implemented and directionally clarified** only if:

1. `oxidase` now has one canonical cross-host example (or an existing one is clearly promoted/refactored into that role)
2. `oxidase-native-runner` is still present as the native-specific proof harness
3. the difference between example and runner is explicit and justified
4. the runner’s durable proof path is either made more reproducible or any remaining path-bound state is explicitly justified
5. the example’s Web and Native paths are both validated honestly

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make the cross-host consumer story visible in an `oxidase` example, while keeping the native-specific proof burden in the runner.
