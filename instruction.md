# Worker Instruction: Implement the Host Ambiguity Guard (`len == 1` implicit, `len >= 2` require `--pid`)

You are working in:

```text
/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host
```

The decision is now fixed:

> **when there is exactly one live host, implicit auto-discovery is allowed; when there are multiple live hosts, silent guessing is forbidden and the caller must specify `--pid`**

Write all agent-facing reasoning in English.

Use this exact reporting structure:

1. **current repo facts**
2. **what I changed**
3. **validation actually run**
4. **final verdict**

Do not overclaim.

---

## 1. Core Goal

Implement a deterministic ambiguity guard for host discovery.

The desired behavior is:

1. **0 live hosts** → clean “not found” error
2. **1 live host** → auto-connect is allowed
3. **2+ live hosts** → auto-connect is rejected; caller must provide `--pid <PID>` (or explicit descriptor/socket path if supported)

This should eliminate silent mis-targeting while preserving zero-friction usage in the single-host case.

---

## 2. Architectural Rule

This policy must live in the **transport/discovery layer**, not only in the CLI.

That means:

> the ambiguity rule should be the transport layer’s single source of truth, so both CLI users and programmatic API consumers get identical behavior.

If the CLI alone enforces the rule, the programmatic client path can still behave differently, which is not acceptable.

---

## 3. Required Behavior

### A. `TargetSelector::Auto`

When the selector is implicit/auto:

1. `0` hosts → `NotFound`
2. `1` host → return that host
3. `2+` hosts → return an ambiguity error listing the competing PIDs and requiring `--pid`

### B. `TargetSelector::Pid(pid)`

When the selector is explicit by PID:

1. connect to that PID if reachable
2. error clearly if that PID is not present / not reachable

### C. Explicit path targeting

If explicit descriptor/socket path targeting already exists, preserve it.

The ambiguity guard is specifically about implicit auto-discovery.

---

## 4. CLI Expectations

The CLI should be updated so that:

1. `blitz-host list` shows available hosts clearly
2. commands using auto-discovery fail fast with a clean error if multiple hosts are active
3. the error explains how to recover:
   - use `blitz-host list`
   - then rerun with `--pid <PID>`

It is acceptable and desirable to centralize repeated connection boilerplate into one helper.

### Preferred structure

Prefer separating:

1. selector determination
2. client connection / ambiguity handling

into clean helper functions rather than copy-pasting connection logic across many subcommands.

---

## 5. Scope Boundary

### Must implement

1. ambiguity guard in transport/discovery
2. PID-based deterministic targeting in CLI paths that connect to a host
3. shared connection plumbing so the rule is applied consistently

### Must not implement

1. `--instance` visible CLI targeting
2. same-process multi-window routing
3. speculative broader target-selection complexity

This pass is specifically about the ambiguity guard and PID disambiguation.

---

## 6. Files / Areas to Reinspect

At minimum:

1. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/handoff.md`
2. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`
3. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/src/discovery.rs`
4. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host-transport/src/client.rs`
5. `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/crates/blitz-host/src/bin/blitz-host.rs`

---

## 7. Validation You Must Run

Run the smallest commands/tests that prove the ambiguity guard actually works.

At minimum:

1. unit tests for discovery behavior:
   - zero hosts
   - one host
   - multiple hosts
   - explicit PID
2. CLI or integration proof showing:
   - implicit success with one live host
   - ambiguity failure with multiple live hosts
   - deterministic success with `--pid`
3. confirmation that existing attach / inspect / action paths still work after the change

If markdown files are edited, validate them.

---

## 8. `result.md` Requirement

Update:

- `/Volumes/HDD-1T-2021-Mac/Vault/business/project/mine/dioxus/util/blitz-host/result.md`

It must explicitly record:

1. where the ambiguity rule lives
2. what happens in the 0 / 1 / many host cases
3. what CLI helper/plumbing was consolidated
4. what validation proved the rule
5. what remains deferred

---

## 9. Final Verdict Rule

You may report **Implemented and clarified** only if:

1. implicit selection works only in the single-host case
2. ambiguous multi-host auto-discovery fails fast and cleanly
3. `--pid` deterministically resolves the target
4. the rule is implemented in the transport/discovery layer, not just the CLI
5. existing control-plane behavior still works

Otherwise report:

- **Partially implemented / still deferred**
or
- **Blocked with evidence**

The purpose of this pass is:

> remove silent multi-host mis-targeting without destroying the convenient single-host workflow.
