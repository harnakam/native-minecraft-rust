# Architecture

## Goal

- Minecraft 1.8.9 only
- Protocol 47 only
- Hypixel PvP focused
- Lightweight, high FPS, low latency
- Use MCP as reference specification, not as code to port line by line

## Non-goals

- Singleplayer
- World generation
- Forge or FML compatibility
- Copying Java source or Minecraft assets into the repository

Full Minecraft 1.8.9 client compatibility is the requested objective. Existing PvP milestones and passing differential fixtures do not establish full compatibility.

## How exact the design should be now

Do not fully lock every internal detail up front. Lock the boundaries that are expensive to change later and the behaviors that directly affect PvP correctness.

### Lock now

- Crate boundaries
- Main-loop structure
- Thread ownership
- Protocol 47 input and output boundary
- Player movement, combat, and inventory behaviors that affect server interaction
- Verification strategy
- Java interop boundary

### Do not lock yet

- Final rendering backend implementation
- ECS vs non-ECS storage choice
- UI widget internals
- Micro-optimizations

The rule is strict external contracts, flexible internal implementation.

## Java reference hotspots

These are the first files to consult. Future work should keep context focused on this set.

- `MCP-919/src/minecraft/net/minecraft/client/Minecraft.java`
  - `runGameLoop` around line 1083
  - `runTick` around line 1741
- `MCP-919/src/minecraft/net/minecraft/util/Timer.java`
  - `updateTimer` around line 65
- `MCP-919/src/minecraft/net/minecraft/client/entity/EntityPlayerSP.java`
  - `onUpdateWalkingPlayer` around line 189
  - `onLivingUpdate` around line 715
- `MCP-919/src/minecraft/net/minecraft/client/multiplayer/PlayerControllerMP.java`
  - `attackEntity` around line 495
  - `windowClick` around line 534
- `MCP-919/src/minecraft/net/minecraft/client/network/NetHandlerPlayClient.java`
  - `handleJoinGame` around line 277
  - `handlePlayerPosLook` around line 669
  - `handleConfirmTransaction` around line 1174
  - `handleKeepAlive` around line 1663
- `MCP-919/src/minecraft/net/minecraft/client/multiplayer/WorldClient.java`
  - `tick` around line 66
  - `addEntityToWorld` around line 234
- `MCP-919/src/minecraft/net/minecraft/client/renderer/EntityRenderer.java`
  - `getMouseOver` around line 409
  - `renderWorld` around line 1280
  - `renderWorldPass` around line 1323
- `MCP-919/src/minecraft/net/minecraft/client/renderer/RenderGlobal.java`
  - `loadRenderers` around line 482
  - `renderEntities` around line 556
  - `setupTerrain` around line 774

## Accuracy priorities

### Must match

- Protocol 47 packet ordering
- Keepalive replies
- Teleport and `PlayerPosLook` acknowledgement behavior
- Sprint and sneak action packet conditions
- Movement packet send conditions
- Held-item synchronization
- Inventory transaction confirmation
- Attack, use, and block-break send order
- Raycast results that affect local prediction

### Can differ at first

- Internal menu architecture
- Visual polish
- Non-PvP effects
- Singleplayer-related systems

## Rust workspace responsibilities

### `rmc-client`

- App startup
- Window integration
- Main loop
- Input collection
- Tick and render orchestration

Keep the main thread close to `Minecraft.runGameLoop`: `input -> fixed ticks -> render -> present`. Do not split this into separate game and render threads unless profiling proves a clear need.

### `rmc-net`

- Protocol 47 codec
- Frozen packet contract modules
- Login, encryption, compression, and play-state transitions
- Inbound packet decode
- Outbound packet queue
- Connection telemetry

The local MCProtocolLib copy is useful as a design reference for codec architecture, but it includes a modern `CONFIGURATION` state and should not be treated as the runtime base for a 1.8.9 client.

The contract-freeze layout for this crate is:

- `protocol/handshake.rs`
- `protocol/login.rs`
- `protocol/play_clientbound.rs`
- `protocol/play_serverbound.rs`

Before implementation expands, each packet definition must freeze:

- Packet ID
- Field order
- VarInt usage
- Compression boundary assumptions

### `rmc-game`

- Local player state
- Local simulation layer
- Movement state machine
- Sprint reset behavior
- Attack, use, block, and inventory interaction rules
- Combat-critical local prediction

This crate owns the behavior currently spread across `EntityPlayerSP` and `PlayerControllerMP`.

The current implementation boundary for M2.5 is:

- authoritative network state
- local simulation state and motion integration
- render interpolation state

Rendering should consume the interpolated snapshot only. Network corrections and knockback should enter through the simulation layer first.

The current M4 gameplay handoff is:

- sprint and sneak state sync through `C0BPacketEntityAction`
- attack and use-item packet sequencing through `C0A`, `C02`, `C07`, and `C08`
- held-item and click-window flow through `C09`, `C0E`, and `C0F`
- local hurt and knockback feedback from `S06`, `S12`, `S30`, and `S32`

### `rmc-world`

- Chunk storage
- Block and entity snapshots
- World diff application
- Lightweight data for visibility and meshing

Because this is Hypixel PvP focused, the world layer consumes server-provided state and does not implement generation.

The current M3 handoff is:

- `PlayClientboundPacket`
- `WorldSnapshot` chunk/block application
- dirty chunk list for the renderer

### `rmc-render`

- Render facade
- Chunk mesh scheduling
- Entity and particle draw submission
- GPU resource lifetime

The important decision now is data flow, not final backend details. Vulkan is the first target on Windows, but the backend stays behind a renderer facade.

The current M3 renderer path is:

- debug chunk bounds for occupancy inspection
- camera-aware chunk visibility culling from the render snapshot
- debug player overlays for the local simulated player and network ghost
- naive opaque-cube chunk meshing for pipeline validation
- dirty chunk rebuild and upload summary tracking

### `rmc-ui`

- HUD
- Chat
- Scoreboard and tab list
- Inventory and chest screens
- Settings screens

Only PvP-relevant UI should be early scope.

The current M5 usability handoff is:

- `rmc-net` decodes chat, sound, window, tab-list, and scoreboard packets into frozen protocol structs
- `rmc-game::inventory` owns open-window metadata, set-slot application, carried-item cursor state, and close-window packet emission
- `rmc-game::usability` merges chat history, sidebar scoreboard state, team formatting, tab-list entries, local settings, and sound cues into a snapshot
- `rmc-ui::PvPHud` turns that snapshot into lightweight text lines so Java-vs-Rust behavior can be diffed before the final GUI renderer exists

The current M6 performance handoff is:

- `rmc-game::tick::FramePacer` smooths raw frame deltas before the fixed-step timer consumes them and emits pacing telemetry
- `rmc-render::ChunkMeshPipeline` now separates `dirty -> scheduled jobs -> completed uploads` so mesh work can be budgeted off the hottest path
- `rmc-render` telemetry snapshots summarize visible chunks, dirty backlog, scheduled rebuilds, cull counts, and upload volume
- `rmc-net::trace::PacketTelemetryHistory` summarizes packet cadence and byte volume from packet trace events
- `rmc-client perf` is the current integration point for measuring pacing and mesh budget regressions before live-server profiling exists

The current completion handoff is:

- `rmc-net::driver::DriverEvent::InboundPlayPacket` exposes decoded clientbound play packets to the main-thread runtime without bypassing the frozen protocol or trace path
- `rmc-client live` is the integrated loop that runs `read socket -> apply packets -> advance shell -> queue outbound packets -> rebuild visible meshes`
- `rmc-world`, `rmc-game::combat`, and `rmc-game::usability` all consume the same inbound packet stream, so world state, PvP state, and HUD state stay on one authoritative path
- `rmc-client live` emits packet summaries and per-frame state lines together, so Java-vs-Rust comparison can happen on an end-to-end session instead of per-milestone harnesses only

### `rmc-java`

- Java-side analysis tools
- Optional sidecar bridge
- Differential testing support

Do not place the JVM on the hot path. If Java is used at runtime, keep it out of render, input, and per-tick combat logic.

## Thread model

### Main thread

- Window events
- Raw input
- Fixed tick
- Render submit
- Present

### Net thread

- Socket read and write
- Packet decode and encode
- Compression and encryption
- Inbound event queue

### Worker pool

- Chunk decode
- Mesh build
- Texture upload staging
- Async IO

## Data flow

1. The net thread decodes packets.
2. Packets become lightweight events.
3. The main thread drains events during tick and applies them to network-facing state.
4. The local simulation layer resolves motion, collision, knockback, and interpolation state.
5. Tick completion updates the render-facing snapshot.
6. The renderer reads that snapshot and does not mutate gameplay state.

Use single-writer ownership and message passing instead of coarse locks on hot paths.

## Implementation strategy

### Source-led reconstruction

Read MCP919 package structure, ownership, inheritance and call order before extending behavior. Mirror its responsibility hierarchy in Rust modules, retaining workspace boundaries where they remain useful. See [MCP919 source map](mcp919-source-map.md). Java inheritance may become Rust composition, but its observable ordering and state ownership must remain explicit. Do not create empty modules to suggest coverage.

### Differential testing first

Treat these as golden paths early:

- Packet trace comparison from M1 onward
- Movement packet sequences
- Teleport correction
- Inventory confirmation
- Hit, use, and swing packet ordering

### Vertical slices

Do not build rendering or UI in isolation first. Each milestone should keep a thin vertical path alive: `join -> move -> render -> interact`.

## Where Java is allowed

Allowed:

- Analysis tools
- Differential comparison
- One-off extraction tools
- Future world-generation sidecar if ever needed

Not allowed on the critical path:

- Per-frame rendering
- Per-tick input or combat logic
- Core serverbound packet decision logic

## Repo policy

- Commit authored code only
- Do not commit decompiled Minecraft source or assets
- Publish authored Rust and analysis tooling only to the user-authorized GitHub repository
- Do not make large binary references the source of truth
