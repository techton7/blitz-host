# Worker Instruction: Implement the Agreed Targeting Model (`list` + `--pid`, with Future Window Readiness)

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The inspection/design discussion already settled the direction.

Do **not** narrow it down incorrectly.

The agreed model is:

1. solve the real **process-level ambiguity** now
2. keep the protocol/design **window-ready** for future same-process multi-window support
3. **do not expose `--instance` as a visible CLI option**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Implement the next `blitz-host` targeting slice so that:

1. users/agents can list running attachable hosts
2. users/agents can explicitly target the correct host process by PID
3. the protocol/descriptors are made ready for future window-level routing
4. today’s single-window default remains ergonomic

This is the actual target.

---

## 2. The Agreed Design You Must Follow

### A. Process-level selection first

Implement the real immediate solution:

1. `blitz-host list`
2. `--pid <PID>`

This solves the current multi-process ambiguity.

### B. Window-level readiness too

Do **not** stop at process selection alone.

Because the runtime inspection already showed that real window/document identity exists:

1. `WindowId`
2. `BaseDocument::id()`

the protocol/design should become ready for future window-level routing now, **with fallback semantics**, even if true same-process multi-window is not live yet.

That means it is acceptable and expected to:

1. extend descriptor metadata with primary window/document identity
2. add optional routing fields (for example `window_id: Option<...>`) to requests where appropriate
3. preserve default fallback behavior when that field is absent

### C. `--instance` must not be a visible CLI option

This is the simplification decision:

> the user-facing CLI should expose `--pid`, not `--instance`

That does **not** mean instance IDs or descriptor paths cannot still exist internally or as lower-level machinery.

It means the human/agent-facing CLI surface should not grow a visible `--instance` option in this slice.

---

## 3. Scope for This Pass

### Must implement

1. `blitz-host list`
2. PID-based selection in the CLI
3. real transport/client selector support for PID targeting
4. descriptor/protocol updates needed to prepare future window-level routing
5. fallback behavior for today’s single-window hosts

### Must not over-expand

1. no full same-process multi-window host registry yet unless it is tiny and unavoidable
2. no broad redesign of the whole protocol beyond what this targeting slice needs
3. no visible `--instance` CLI option

Keep it to the agreed model.

---

## 4. Current Facts You Should Start From

Unless reinspection disproves them:

1. the current practical runtime is one process → one primary window/document
2. the real immediate collision surface is multiple host processes
3. Blitz/Winit still already allocate real window/document identities
4. the right next step is therefore:
   - process-level UX now
   - window-level protocol readiness now
   - full same-process multi-window execution later

This pass should implement exactly that.

---

## 5. Required UX

### `blitz-host list`

Add a command that lists live reachable hosts in a useful way.

At minimum, include enough fields to make targeting decisions practical.

Reasonable fields include:

1. PID
2. renderer/app name
3. renderer/app version
4. primary document ID
5. primary window identity if exposed
6. socket path / status as useful

### `--pid <PID>`

Expose `--pid` on the relevant commands such as:

1. `inspect`
2. `click`
3. other current host-connecting commands if appropriate

Behavior:

1. if `--pid` is present, connect to that process deterministically
2. if omitted, keep today’s default discovery behavior

### No visible `--instance`

Do not add `--instance` to the visible CLI surface in this pass.

---

## 6. Transport / Client Requirements

PID targeting must be real all the way down, not a CLI-only trick.

That means you should add or update an actual selector model in code, for example:

1. `TargetSelector::Pid(u32)`
2. `DebugClient::connect_to(selector)`
3. or an equivalent clean abstraction

If instance IDs remain useful internally, they may stay internal. They do not need to be a user-facing flag.

---

## 7. Protocol / Descriptor Readiness

This pass should also prepare the protocol/design for future window-level routing.

That may include:

1. surfacing primary window/document identity in descriptors
2. introducing optional request routing fields with primary fallback semantics

Important:

> today’s common path must remain ergonomic, meaning omission should naturally target the current primary/default window.

Do not force users to specify a window identifier in the single-window case.

---

## 8. What Not to Do

1. do not expose `--instance`
2. do not fully implement same-process multi-window host registries yet
3. do not pretend process-level selection alone finishes the architectural question
4. do not overcomplicate the visible UX for today’s single-window case

This pass should implement the agreed middle path.

---

## 9. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/ROADMAP.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/src/client.rs`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/src/discovery.rs`
6. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-protocol/`
7. any descriptor and request types affected by targeting changes

---

## 10. Validation You Must Run

Run the smallest commands that prove the targeting model actually works.

At minimum:

1. validate `blitz-host list`
2. validate `blitz-host inspect --pid <PID>`
3. validate `blitz-host click <NODE_ID> --pid <PID>` if click remains supported
4. validate the client/transport selector implementation behind PID targeting
5. rerun the current proof path to ensure attach / inspect / click still work

Do not call this complete based only on compile success.

If markdown files are edited, validate them.

---

## 11. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. how `list` works
2. how `--pid` targeting works
3. what transport/client selector was added
4. what descriptor/protocol readiness was added for future window-level routing
5. how fallback-to-primary behavior works today
6. what remains deferred

---

## 12. Final Verdict Rule

You may report **Implemented and clarified** only if:

1. `blitz-host list` works
2. PID-based targeting is real and deterministic
3. the CLI and client/transport layer agree on the selector model
4. the protocol/design is more future-ready for window routing without breaking today’s ergonomic default path
5. the existing attach / inspect / click flow still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> implement the agreed targeting model: process-level selection now, future window-level readiness built in, and no visible `--instance` option.
