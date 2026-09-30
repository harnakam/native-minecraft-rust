# Verification

## Commands

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo build -p rmc-client --locked
python scripts/audit_publication.py
cargo run -p rmc-client -- verify live-suite --mcp-root MCP-919
```

The live suite requires a local MCP919 installation and Java. It instruments the local reference client and captures both clients against a scripted server. Reference sources, reports and captures remain ignored.

## Observed results (2026-09-30 / 2026-10-01, Windows)

The official 1.8.9 server JAR was checked against its published object SHA1, then run locally after the user confirmed EULA acceptance. Offline compressed login, join, chunk reception, windowed rendering and a ten-second forward/sprint run succeeded. The window smoke received 49 chunks and advanced 118 ticks. The movement run advanced 200 ticks and received 77 chunks without a server disconnect.

The strict live comparison passes: packet 54 Rust / 54 Java records; movement 35 / 35; combat 3 / 3; inventory 5 / 5, with zero differences. The movement capture includes the initial correction and 34 local ticks. Initial-position readiness, script timing and swing-before-attack ordering were corrected. Packet records retain their order within each TCP direction; scheduling order between independent directions is normalized. These counts describe scripted coverage, not overall parity.

Old comparison logic discarded most records and vertical coordinates. It has been removed. All records and six movement coordinates are retained, empty traces fail, and old comparison reports cannot open the compatibility gate. The current scripted reports open that gate; this only permits further testing and does not certify Hypixel gameplay, which remains unverified.

## Local collision and travel probes

```sh
python scripts/verify_local_oracles.py --mcp-root MCP-919 --java-home /path/to/jdk8
```

The command compiles only the authored `RmcCollisionOracle` and `RmcMovementOracle` helpers against an already compiled local MCP installation. Java classes, assets and reports remain local and ignored. Rust dependencies must be cached for its offline Cargo calls. Supply `--cargo /path/to/cargo` when needed.

Observed: 3,039 valid block/metadata collision cases match by complete union of AABB volumes. Invalid Java metadata states are excluded; this probe places one block without neighbors and does not verify connected geometry or moving piston tile entities. Twenty-one travel cases (walk, diagonal, sprint, held jump, ice, soul sand, slime, water, Depth Strider I/III/over-level water, enchanted lava, water jump, shallow water, flowing water, water edge exit, falling water, lava, ladder, web and slab stepping) match 1,680 ticks in position, velocity and ground state at tolerance 2e-7. The travel probe invokes the actual Java player/living movement methods with directly controlled inputs; it does not cover every client input transition, enchantment, entity interaction or fluid flow configuration.

Death and respawn are exercised separately over a local TCP connection: server health zero, twenty tick button delay, exactly one serverbound Client Status action zero, then a server Respawn clears death state. The window exposes respawn and leave controls; hardcore suppresses survival respawn. This does not yet verify every death-screen detail or cross-dimension state reset.

The authored selection probe calls real Java ray methods for forty-two rays per valid isolated block metadata state. Including origins inside blocks and on their boundaries, all 127,638 ray hit/miss, distance and face observations match. Reference-invalid metadata is excluded. Connected fences, panes and walls additionally match 69,696 collision/selection-volume observations: eleven target types, all 198 neighboring types at metadata zero, sixteen horizontal connection masks, and both bounds kinds. Mixed neighboring types, stair/door/chest combinations and tile entities still need broader fixtures.

Mining speed matches 26,136 exact IEEE single-precision observations: 198 blocks, hand plus 21 tools, and normal/airborne/efficiency/haste/fatigue/underwater conditions. Lookup rules store only measured numeric behavior and registry identifiers. The controller probe invokes the actual `PlayerControllerMP`; audio, graphics bootstrap and packet transport use test adapters. Seven scenarios match 140 ticks of damage, activity, predicted block state and digging actions: stone, target switch, release, bedrock, instant slime, creative and creative sword. This does not verify all special mining interactions, durability/enchantment randomness or full input/tick batching.

The official-server survival test was run successfully in an isolated localhost server directory: prepare a stone target and diamond pickaxe, mine through the live runtime, request `/testforblock ... air` and require the server's success response, then `/kill` and request respawn. It requires a local operator named `VanillaProbe`, an offline test server and prior EULA agreement. It modifies only that test world's blocks and player state. It is opt-in because neither a Minecraft server nor game assets are distributed in CI:

```sh
RMC_VANILLA_PORT=25570 cargo test -p rmc-client official_server_confirms_mining_and_respawn -- --ignored --nocapture
```

In PowerShell, set `$env:RMC_VANILLA_PORT='25570'` before running the Cargo command. The locally started probe server was stopped after the test.

## Limits

Unit tests do not establish complete Minecraft compatibility. Additional fixtures must cover liquids, ladders, effects, complex block geometry, specialized inventory slots, respawn/dimension changes and real online multiplayer sessions. Authentication secrets must never appear in captures or commits.
