# Blitz Host Roadmap

This document is the implementation roadmap for `util/blitz-host/`.

Use this file for:

- crate boundaries
- implementation sequencing
- scope control for the first internal release
- separating reusable protocol/transport work from Blitz-specific bridge work

Reference-source inspection lives under:

- `util/blitz-host/reference/`

Those references inform this roadmap, but they are **not** product dependencies.

## Mission

Build an internal-only control surface that lets an agent attach to and operate an **already-running Blitz window** safely on the local machine.

The first goal is not “public remote debugging for everything.”

The first goal is:

> give our internal native Blitz runner a real, local-only, agent-accessible control/debug lane with typed requests, explicit UI-thread handoff, and trustworthy inspection/action behavior.

## Current direction

The current architectural direction is:

1. keep `oxidase` focused on runtime/frame/bootstrap ergonomics
2. keep running-window agent/debug control **outside** `oxidase`
3. treat `blitz-host` as an internal utility lane first
4. design around **three explicit layers**

## Confirmed boundaries

### `blitz-host` owns

- typed control protocol for live-window inspection/actions
- local transport and discovery
- UI-thread bridging into a running Blitz/Dioxus Native host
- debug-control lifecycle wiring for internal runners

### `oxidase` does not own

- socket servers
- debug authentication/discovery tokens
- running-window remote control
- transport policy

### First host target

The first real host target is:

- `util/oxidase/crates/oxidase-native-runner`

in the `blitz-host` feature-enabled development lane.

## Reference-informed architectural shape

The reference inspection supports this crate layout:

```text
util/blitz-host/
  crates/
    blitz-host-protocol/
    blitz-host-transport/
    blitz-host-bridge/
```

### 1. `blitz-host-protocol`

Pure typed vocabulary.

It should own:

- request/response/event types
- semantic-node and snapshot data models
- action enums
- JSON serialization/schema-facing types

It should **not** depend on:

- Blitz
- Winit
- Dioxus Native
- Tokio

### 2. `blitz-host-transport`

Local transport and discovery.

It should own:

- local server/client transport
- discovery descriptor
- owner-only file/socket permissions
- wake-up plumbing interface
- session or attach policy

First preference:

- Unix domain socket + owner-only descriptor on macOS/Linux

Possible later fallback:

- loopback TCP + token-based attach

### 3. `blitz-host-bridge`

The running-host adapter.

It should own:

- adapting a real running Blitz document/window to the protocol
- UI-thread handoff
- synthetic action dispatch
- DOM/layout/semantic inspection
- quiescence/settlement after actions

It is expected to depend on:

- current upstream Blitz/Dioxus Native lane used in this workspace

This layer is where most host-specific code belongs.

## V1 scope

The first version should be intentionally narrow.

### Required in V1

1. internal-only lane
2. attach to an already-running native Blitz window
3. local-only transport
4. typed inspect requests
5. typed action requests
6. explicit UI-thread handoff
7. post-action settlement/quiescence
8. integration with `oxidase-native-runner`

### Nice to have in V1 if cheap

1. capture/screenshot support
2. basic lifecycle request such as `quit`
3. a small descriptor file for discovery

### Not required in V1

1. public crates.io release
2. broad cross-platform transport parity
3. full WebDriver compatibility
4. generalized multi-window orchestration
5. large diagnostics surface
6. public API stabilization

## Current established state

The roadmap is no longer at the original inspect-only hypothesis stage.

The currently established state is:

1. `blitz-host` attach / inspect is runtime-proven
2. `blitz-host` click / settle / changed-state verification is runtime-proven
3. the ergonomic integration boundary now lives at `oxidase`
4. in the feature-enabled development lane, `blitz-host` availability is on by default without requiring a positive `--debug-control` flag
5. `oxidase` now has a canonical cross-host example
6. `oxidase-native-runner` remains the native-specific proof harness

This means the roadmap should now treat the following as the active split:

- **canonical cross-host consumer story** → `crates/oxidase/examples/cross_host/`
- **native-specific proof harness** → `crates/oxidase-native-runner/`

The runner should not be treated as the only proof of value anymore.

## Non-goals

These are explicitly out of scope for the first pass:

1. merging this into `oxidase`
2. cloning PathScale’s document layer directly
3. exposing network-accessible remote control outside the local machine boundary
4. building a production-facing browser/debug server
5. solving every future host at once

## Implementation sequence

### Phase 0 - reference lane

Status:

- reference acquisition and inspection completed under `util/blitz-host/reference/`

Exit condition:

- enough source evidence exists to define the crate split and V1 scope

### Phase 1 - protocol crate

Build `blitz-host-protocol`.

Focus:

- typed request/response vocabulary
- small, stable semantic models
- minimal action surface for V1

Exit condition:

- protocol types compile independently with lightweight dependencies only

### Phase 2 - transport crate

Build `blitz-host-transport`.

Focus:

- local-only server/client
- descriptor/discovery format
- owner-only permissions
- host wake-up interface

Exit condition:

- a host can start a control transport and a client can discover/connect locally

### Phase 3 - bridge crate

Build `blitz-host-bridge`.

Focus:

- attach to current Blitz/Dioxus Native host lane
- inspect semantic/layout state
- dispatch actions on the correct thread
- settle after actions before reporting stable results

Exit condition:

- typed requests can read and act on a real running document through the bridge

### Phase 4 - runner integration

Integrate into `oxidase-native-runner`.

Focus:

- durable native proof harness behavior
- no regression in attach / inspect / click / settle proof
- keeping the harness distinct from the canonical public example story

Exit condition:

- live internal runner can be controlled/inspected locally

### Phase 5 - example boundary clarification

Establish the public-facing cross-host consumer story.

Focus:

- one canonical `oxidase` example representing the same app code on Web and Native
- clear separation between example and native proof harness
- honest documentation of the runner as an internal harness

Exit condition:

- the canonical example and the harness have distinct, explicit roles

### Phase 6 - optional expansion

Only after V1 is real:

- richer diagnostics
- capture/image responses
- lifecycle extensions
- stronger discovery/session behavior
- broader host coverage

## Recommended immediate execution slice

The roadmap already assumes the full V1 path, but the **next actual coding slice**
should be narrower than full V1.

The recommended next slice is:

1. create the internal `crates/` workspace structure
2. implement `blitz-host-protocol` for the smallest useful request/response set
3. implement `blitz-host-transport` for local discovery and local attach
4. implement the first `blitz-host-bridge` vertical slice for **read-only inspect**
5. wire that inspect-only path into `oxidase-native-runner` behind `--debug-control`

### Why this was the right first stop line

This proves the most important architectural question first:

> can an agent discover and attach to a real running Blitz window and read a stable typed snapshot without disturbing the runtime?

That gives us an end-to-end proof before we take on the riskier pieces:

- synthetic action dispatch
- settlement after mutation
- capture/image responses
- lifecycle commands

### What to defer until after inspect-only proof

Defer these to the next slice unless they turn out to be nearly free:

1. `act`
2. `capture`
3. `quit` / lifecycle controls
4. richer diagnostics streams
5. multi-client behavior

In short:

> first prove **attach + inspect** end to end, then add **act + settle**, then clarify the `oxidase` boundary, then establish the canonical cross-host example, then move on to broader expansion.

## Next expansion categories

Now that the initial attach / inspect / act / settle / `oxidase`-boundary / example split work is established, the roadmap should treat the next steps as three explicit expansion categories:

### Category 1 - multi-window targeting & discovery UX

This is now the most immediate usability gap.

Focus:

1. identifying which live Blitz window is which
2. better descriptor metadata (for example window/app identity)
3. explicit selection UX (`list`, `--pid`, instance selection, descriptor targeting)
4. making multiple simultaneously-running windows practical for agent control

### Category 2 - richer control / input surface

The current proven interaction is click-only.

Next likely additions:

1. focus
2. set-value / text input
3. keyboard actions
4. hover / pointer / scroll as needed

The goal is to move from “can click a button” toward “can drive real app workflows.”

### Category 3 - capture & richer diagnostics

After targeting and richer control improve, the next major value lane is observability beyond semantic DOM:

1. visual capture / screenshot support
2. richer diagnostics streams
3. eventually stronger rendering/debug proof surfaces

This is where `blitz-host` becomes more useful for debugging visual/native rendering issues that DOM inspection alone cannot prove.

## Security posture

The first security posture should be deliberately simple and local:

1. local-machine only
2. owner-only filesystem permissions for descriptors/sockets
3. debug-only / feature-gated / flag-gated activation
4. no claim that this is a sandbox against the same OS user

If a transport requires authentication beyond filesystem access, keep it narrow and explicit.

## Validation expectations

At minimum, V1 validation should prove:

1. the transport can start and advertise locally
2. the runner can attach the bridge without breaking ordinary execution
3. inspect returns a stable typed response from a real running window
4. action dispatch occurs on the correct host/UI thread
5. post-action inspection observes settled state rather than stale state

## One-sentence summary

> `blitz-host` should become an internal three-layer control stack — protocol, transport, and bridge — that gives agents local access to already-running Blitz windows without expanding `oxidase` beyond its runtime role.
