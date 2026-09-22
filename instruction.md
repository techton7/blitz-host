# Worker Instruction: Move `blitz-host` Integration Up to the `oxidase` Ecosystem Boundary

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current `blitz-host` spike/facade has proven that local attach, inspect, click, and settle work.

The next architectural move is now fixed:

> **`blitz-host` should remain its own project/crate family, but the ergonomic integration point should move up to the `oxidase` ecosystem boundary.**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Refactor the integration direction so that:

1. `blitz-host` remains structurally separate
2. `oxidase` becomes the place where the convenience integration is exposed
3. downstream Dioxus Native / Blitz hosts do **not** need to depend on `blitz-host` directly if they are already choosing the `oxidase` path
4. `dioxus-native-dom` does **not** directly depend on `blitz-host`

The desired end-state for `oxidase` ecosystem consumers is:

```toml
oxidase = { git = "https://github.com/techton7/oxidase.git", tag = "oxidase-vX.Y.Z", features = ["blitz-host"] }
```

and then ordinary:

```rust
#[oxidase::main]
fn main() {
    dioxus::launch(App);
}
```

with runtime activation via:

- `--debug-control`
- or environment variable equivalent

That is the model this pass should push toward.

---

## 2. Critical Architectural Rule

This is the key rule:

> **`dioxus-native-dom` must not grow a direct dependency on `blitz-host`.**

If host/runtime convenience glue is needed, it belongs:

1. in `oxidase`
2. or in an `oxidase`-side integration/glue layer

but **not** as a direct hard dependency inside `dioxus-native-dom`.

You may rely on `dioxus-native-dom` APIs or add narrowly-scoped upstream-friendly hooks only if absolutely necessary, but do not turn it into a `blitz-host` consumer.

---

## 3. Current Facts You Should Start From

Unless reinspection disproves them:

1. `blitz-host` now has:
   - protocol
   - transport
   - bridge
   - facade crate
   - working CLI
   - proven attach / inspect / click / settle flow
2. The current Dioxus Native host proof still relies on explicit host integration surfaces.
3. The remaining structural problem is not technical capability, but **where the integration responsibility should live**.
4. The desired answer is now:
   - `blitz-host` stays separate
   - `oxidase` owns the developer-facing convenience layer

---

## 4. Required Direction

### A. Keep `blitz-host` separate

Do **not** collapse `blitz-host` into `dioxus-native-dom`.

Do **not** turn `dioxus-native-dom` into the package that owns this feature.

`blitz-host` remains its own project/crate family.

### B. Move convenience integration up into `oxidase`

Push the integration story so that `oxidase` (or an `oxidase`-side glue surface) becomes the intended ergonomic entrypoint.

That means this pass should work toward:

1. `oxidase` having a feature such as `blitz-host`
2. `#[oxidase::main]` becoming the natural place where the integration is consumed
3. runtime `--debug-control` deciding activation

### C. Git/tag dependency story must be real

Do not describe this as path-only local magic.

The intended downstream consumption story must be compatible with a real GitHub dependency/tag lane, for example:

```toml
oxidase = { git = "https://github.com/techton7/oxidase.git", tag = "oxidase-v0.1.4", features = ["blitz-host"] }
```

or the next correct tag/version if it changes during your work.

The point is:

> a downstream host should be able to opt into `blitz-host` ergonomics by opting into `oxidase`, not by directly stitching together raw `blitz-host-*` crates.

---

## 5. What This Pass Must Clarify in Code/Design

You must make the following explicit and truthful:

1. what remains inside `blitz-host`
2. what moves to the `oxidase` integration surface
3. how `#[oxidase::main]` would participate when the feature is enabled
4. what still blocks true generic non-`oxidase` single-init integration
5. why `dioxus-native-dom` should stay dependency-clean with respect to `blitz-host`

If you can implement real code toward this direction in the current workspace, do so.

If some parts are not yet implementable cleanly without broader changes, make the limits explicit rather than pretending they are done.

---

## 6. Preferred Outcome

The preferred outcome of this pass is one of:

### Best case

You actually land the beginning of the real `oxidase`-side feature-gated integration surface.

### Acceptable case

You materially restructure the current code and docs so that:

1. `blitz-host` is clearly the internal engine/tooling project
2. `oxidase` is clearly the intended downstream ergonomic boundary
3. the dependency model and feature model are explicit and honest

But do not claim actual `#[oxidase::main]` automatic feature-based injection is complete unless it truly is.

---

## 7. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/oxidase/crates/oxidase-macro/`
7. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/blitz/packages/dioxus-native-dom/`

---

## 8. Validation You Must Run

Run the smallest commands that honestly validate the work you actually land.

At minimum:

1. relevant `cargo check` / `cargo test` for `util/blitz-host`
2. relevant `cargo check` / `cargo test` for `util/oxidase` if you touch it
3. proof that the existing attach / inspect / click / settle flow still works if your changes affect the current working path

Do not call this complete based only on design prose.

If markdown files are edited, validate them.

---

## 9. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. whether any real `oxidase`-side integration code was added
2. what the intended dependency line is (`oxidase` feature + Git/tag dependency)
3. why `dioxus-native-dom` does or does not need to know about `blitz-host`
4. what remains inside `blitz-host`
5. what still remains unresolved

---

## 10. Final Verdict Rule

You may report **Implemented and directionally integrated** only if:

1. the `blitz-host` / `oxidase` boundary is materially clearer than before
2. the dependency story is made truthful
3. `dioxus-native-dom` is kept free of direct `blitz-host` dependency
4. the report clearly distinguishes current implementation from target architecture

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> shift `blitz-host` from being a direct consumer burden toward becoming an optional `oxidase` ecosystem feature, while keeping `dioxus-native-dom` dependency-clean.
