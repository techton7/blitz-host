# Worker Instruction: Simplify `capture` to Required File Output + Metadata JSON Only

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The current capture lane works, but its interface is still too heavy.

The new direction is fixed:

> **`capture` should always require an output file path and should return metadata JSON only — no inline base64 payloads, no fallback behavior**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Simplify the `blitz-host capture` contract so that it behaves like a clean artifact-producing command.

The intended model is:

1. user must provide `-o/--output <FILE>`
2. capture always writes the artifact to disk
3. stdout returns only metadata JSON
4. inline base64 image payload support is removed

This applies to both:

1. full-window capture
2. node/subtree crop capture

---

## 2. Required Changes

### A. CLI behavior

Update the `blitz-host capture` command so that:

1. `-o/--output` is **required**
2. omitting it is a usage error
3. there is no default fallback filename generation
4. there is no inline-image-on-stdout behavior

The CLI should clearly communicate the requirement in its help output and errors.

### B. JSON output contract

The JSON output should remain always-on, but it should be **metadata JSON only**.

Reasonable fields include:

1. `success`
2. `width`
3. `height`
4. `format`
5. `filePath`
6. `nodeId` or equivalent when node capture is used
7. `message`

The image bytes themselves should not be included in the JSON response.

### C. Client / protocol surface

If the current wire model is built around base64 image data:

1. remove or deprecate that inline payload
2. align client APIs with file-oriented or raw-byte-oriented internal behavior as appropriate
3. make sure the public story is consistent with the CLI simplification

Do not leave hidden legacy payloads pretending to still be part of the supported contract if they are not.

---

## 3. Explicitly Remove

Remove these behaviors from the current supported `capture` story:

1. `data_base64` in the public JSON response
2. `--json` meaning “inline the PNG blob”
3. default file path fallback behavior for `capture`

If internal helpers still need raw bytes before file write, that is fine.

The point is:

> the **public contract** should be file artifact + metadata JSON only.

---

## 4. What Not to Break

Do not break:

1. full-window capture functionality
2. node/subtree crop capture functionality
3. the rest of the control-plane proof path
4. deterministic JSON stdout behavior

The simplification should change interface shape, not actual capture correctness.

---

## 5. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/handoff.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/README.md`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
7. any capture-specific helpers in the bridge/client layers

---

## 6. Validation You Must Run

Run the smallest commands that prove the simpler contract works.

At minimum:

1. validate `blitz-host capture -o <FILE>` for full-window capture
2. validate `blitz-host capture --node <ID> -o <FILE>` (or equivalent) for node/subtree capture
3. confirm stdout is valid metadata JSON only
4. confirm no inline base64 payload remains in the public CLI contract
5. rerun the relevant test suite(s) so the simplified capture interface remains green

If markdown files are edited, validate them.

---

## 7. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what public capture surface changed
2. that `-o/--output` is now required
3. that inline base64 JSON payload support was removed
4. what metadata JSON now looks like
5. what validation proved the new contract
6. what remains deferred

---

## 8. Final Verdict Rule

You may report **Implemented and simplified** only if:

1. `capture` requires an explicit output path
2. metadata JSON remains valid and useful
3. inline base64 output is removed from the public contract
4. capture still works for full-window and node/subtree cases

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make `capture` a clean artifact-producing command instead of a mixed file/blob interface.
