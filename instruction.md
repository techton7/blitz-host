# Worker Instruction: Finish the `capture` Simplification for Real (Remove Inline Base64 Completely)

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The previous pass made capture green, but it did **not** fully satisfy the requested contract simplification.

The current problem is:

> **`data_base64` is still present in the protocol, bridge, client, tests, and result narrative**

This pass must remove that, not just hide it.

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Finish the `capture` contract simplification completely.

The intended end-state is:

1. `capture` always requires `-o/--output`
2. the public JSON response is metadata-only
3. inline base64 image payloads are removed from the supported contract entirely

That means:

> not just “CLI hides base64,” but “the contract no longer revolves around `data_base64`.”

---

## 2. What Is Still Wrong

The previous state is insufficient because:

1. `CaptureResponse.data_base64` still exists
2. bridge code still base64-encodes PNG bytes
3. client helpers still decode `resp.data_base64`
4. tests still assert on `data_base64.len()`
5. result text still describes base64-based responses

That is not the requested simplified model.

---

## 3. Required Contract

The public capture contract must become:

### CLI

```bash
blitz-host capture ... -o <FILE>
```

with:

1. required `-o/--output`
2. metadata JSON on `stdout`
3. actual image bytes written to the requested file path

### Public JSON

The returned JSON should describe the artifact, for example:

1. `success`
2. `filePath`
3. `width`
4. `height`
5. `format`
6. `nodeId`
7. `bytes`
8. `message`

The actual PNG data should **not** be embedded in the JSON.

---

## 4. Required Removal

Remove `data_base64` from the supported capture contract.

That means you must inspect and update all relevant layers:

1. protocol types
2. bridge response assembly
3. client helpers
4. CLI plumbing
5. tests
6. result/docs

If internal code still needs raw bytes temporarily before writing the file, that is fine.

But the public response and public flow must no longer expose or rely on base64 image payloads.

---

## 5. What Not to Do

1. do not merely stop printing base64 while keeping it as the hidden contract center
2. do not leave legacy fields in types “just in case”
3. do not weaken tests so the issue disappears without the contract actually changing

This pass is about removing the old model, not cosmetically papering over it.

---

## 6. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/handoff.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/README.md`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-bridge/`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
7. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`

---

## 7. Validation You Must Run

Run the smallest commands that prove the simplified contract is real.

At minimum:

1. rerun the `blitz-host` suite
2. validate full-window capture with required `-o`
3. validate node/subtree capture with required `-o`
4. confirm metadata JSON no longer contains inline image payload
5. confirm existing capture functionality still works

If markdown files are edited, validate them.

---

## 8. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. that `data_base64` was removed from the supported capture contract
2. what the new metadata-only JSON looks like
3. what internal flow now writes the artifact
4. what validation proved the change
5. what remains deferred

---

## 9. Final Verdict Rule

You may report **Implemented and simplified** only if:

1. `data_base64` no longer exists as part of the supported public capture contract
2. metadata JSON is the only stdout payload
3. required `-o/--output` works for both full-window and node capture
4. existing capture functionality still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> finish the capture API simplification all the way, not halfway.
