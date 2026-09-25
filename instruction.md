# Worker Instruction: Add `inspect -o` and Fix DOM-Standard Click Resolution Semantics

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The next `blitz-host` pass now has two tightly-related goals:

1. make `inspect` capable of spilling large JSON snapshots to a file via `-o/--output`
2. fix click resolution semantics so that valid DOM targets without active Dioxus listeners succeed as unhandled clicks instead of hard errors

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Implement two related improvements:

### A. `inspect -o <PATH>`

Allow large inspect results to be written to disk while returning a compact metadata summary on `stdout`.

### B. DOM-standard click resolution

Separate:

1. **target does not exist** → error
2. **target exists but no Dioxus listener handled the click** → success, but explicitly marked as unhandled

The goal is to make `blitz-host` more faithful to real DOM/UI interaction semantics and more usable for humans and agents.

---

## 2. Required `inspect -o` Behavior

### Default behavior

When `-o/--output` is **not** provided:

1. preserve current inspect behavior
2. emit the full inspect JSON to `stdout`

### `-o/--output` behavior

When `-o/--output <PATH>` is provided:

1. write the full pretty JSON snapshot to the given file
2. emit only compact metadata JSON on `stdout`

Reasonable metadata fields include:

1. `success`
2. `filePath`
3. `rootId`
4. `nodeCount`
5. `bytes`
6. `message`

This should help with:

1. terminal readability
2. smaller agent context usage
3. cleaner CI logs

`inspect -o` should remain optional, not mandatory.

---

## 3. Required Click Resolution Policy

### Distinguish target existence from listener handling

The current/desired model must clearly separate:

1. **Selector / node does not resolve to a real DOM node**  
   → error
2. **Node exists, click dispatched, Dioxus listener handled it**  
   → success
3. **Node exists, click dispatched, but no Dioxus listener handled it**  
   → still success, but clearly marked as unhandled

### Why this matters

This is needed for legitimate UI automation such as:

1. outside clicks
2. clicking `body` / background containers
3. blur dismissal flows
4. future host surfaces where valid DOM nodes exist outside the Dioxus VDOM listener subtree

Do not conflate “no listener handled the event” with “target is invalid.”

---

## 4. Preferred Response Shape

If possible, do not encode the handled/unhandled distinction only in a message string.

Prefer a structured field such as:

1. `handled: bool`
or
2. `dispatchState: "handled" | "unhandled"`

If that is too invasive for this pass, document clearly why you kept a message-based status instead.

The goal is to make the success state machine-readable, not just human-readable.

---

## 5. Scope Boundary

### Must implement

1. optional `inspect -o`
2. valid-node / unhandled-click success semantics
3. proof that the new behavior works

### Must not over-expand

1. do not redesign the whole response model unless needed
2. do not turn this into generalized event bubbling work across every action type unless the change is naturally shared
3. do not change unrelated capture behavior in this pass

Keep it to the two agreed improvements.

---

## 6. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/handoff.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/README.md`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/host.rs`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/`
7. any protocol types affected by the click status semantics or inspect metadata summary

---

## 7. Validation You Must Run

Run the smallest commands that honestly prove the new behavior.

At minimum:

1. verify `inspect -o <PATH>` writes the full JSON file and returns compact metadata JSON
2. verify ordinary `inspect` still prints full JSON to `stdout`
3. verify a valid selector such as `"body"` or another valid static container succeeds even if no Dioxus listener handles the click
4. verify a missing selector/node still fails cleanly
5. rerun the relevant `blitz-host` tests

If markdown files are edited, validate them.

---

## 8. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. what `inspect -o` now does
2. what compact metadata JSON looks like
3. how click resolution now distinguishes invalid vs unhandled vs handled targets
4. whether handled/unhandled is exposed structurally or only in message text
5. what validation proved the change
6. what remains deferred

---

## 9. Final Verdict Rule

You may report **Implemented and clarified** only if:

1. `inspect -o` works as specified
2. valid-but-unhandled click targets are no longer treated as hard failures
3. missing targets still fail cleanly
4. the result is documented honestly

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> make `blitz-host` less noisy for inspect consumers and more correct about what a successful click actually means.
