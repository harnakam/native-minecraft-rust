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

Respawn regression checks preserve received chunks within the same dimension, discard them when changing dimension, and recreate local movement and inventory state. The official-server mining/death/respawn test was rerun successfully after this correction.

The authored S27 probe matches twelve exact wire/decoded-position records across four positive/negative/large-center cases. Explosion reception removes affected blocks, invalidates their meshes and adds player velocity in received packet order; regression checks cover position corrections before/after the explosion. Explosion sounds, particles and rendering feedback remain unimplemented. The opt-in official-server test can stop its authorized test server after success with `RMC_STOP_TEST_SERVER=1`.

Local flight controls now implement the MCP seven-tick double-jump window, preserve received capability/speed fields in C13, notify before the tick's movement packet, and disable flight on landing outside spectator mode. Input-to-packet regression tests cover holding jump, expiry and server denial. These control tests are source-derived; a full EntityPlayerSP differential flight capture and spectator no-clip parity remain outstanding.

Number-key inventory swaps now send mode 2 with a null return stack, update player/container aliases locally and roll back rejected transactions. Tests cover occupied main/armor slots, full inventory refusal and displaced-stack merging. The official-server test confirms both swap destinations through `/testfor` player inventory NBT before mining. Recipe-result swaps are explicitly unavailable until local recipe side effects are implemented.

## Container throw (MCP919 mode 4)

The native inventory GUI now maps Q to throwing one item and Ctrl+Q to throwing the hovered stack. `inventory/container.rs` follows Container.slotClick: an occupied cursor prevents the throw and the click return stack is null. Prediction uses the existing confirmation/rollback path and synchronizes player inventory aliases in open containers. Pickup and number-key click algorithms reside in the same module.

Three focused tests cover one/whole-stack prediction, rejection restoration, cursor gating, invalid slots and chest/player alias restoration. The opt-in official-server test additionally installs twelve stones in player slot 9, sends each throw through LiveRuntime/TCP, and checks server Inventory NBT for eleven stones and then absence. This covers ordinary player storage, not recipe outputs or custom container take permissions. Recipe-result throws are explicitly rejected until pickup side effects are implemented. Dropped-item entity rendering remains incomplete.

Creative middle-click cloning (mode 3) now uses the same container module and live TCP path. It copies the selected item including NBT, sets its item-specific maximum stack count, requires creative mode and an empty cursor, and returns null. A focused test checks survival gating, occupied cursor behavior and 64/1 stack limits. Creative cloning has not yet been exercised against the official server.

## Shift-click transfer

`inventory/container.rs` implements mode 1 for player storage and ordinary chest storage, wired to Shift+left/right click in the native GUI. The source inspection covered Container.slotClick, mergeItemStack, ContainerChest.transferStackInSlot and ContainerPlayer.transferStackInSlot. Existing stacks are merged before selecting an empty slot; chest-to-player order is reversed, and player transfers auto-equip armor before moving between main storage and hotbar. Return stacks preserve the original count, including partial transfers. Prediction uses transaction rollback and player/container aliases.

Four tests cover reverse-order merge, full destinations, partial transfers and armor rollback. The opt-in official-server test confirms main-to-hotbar and reverse transfers using server Inventory NBT. Specialized containers and recipe-result side effects remain unsupported. The transfer merge predicate now uses observed item subtype properties, so metadata is compared only for items with subtypes. No complete inventory compatibility claim follows from these tests.

## Complete registered item property observations

RmcItemPropertiesOracle queries the local MCP919 registry for maximum stack size, subtype status and maximum damage. Rust stores only these numeric observations in inventory/item_properties.rs. The `--only item_properties` probe compares the complete 337-item registry, including IDs and all three properties, with zero differences. The comparator rejects incomplete, duplicated and malformed observations; two Python integrity tests exercise those failures.

Stack limits now come from the observed registry, replacing a manually maintained list. Shift-click and displaced-hotbar insertion ignore metadata for non-subtype items; normal valid-slot pickup continues to require exact metadata as Container.slotClick does. A regression checks paper merging and wool color separation. This validates registered properties and selected merge behavior, not all inventory or item behavior.

## Specialized storage transfer algorithms

`inventory/transfer.rs` owns destination selection corresponding to the individual MCP919 container classes. Hopper, dispenser and dropper use lower-inventory/player transfer ranges. Beacon transfers insert payment only when the payment slot is empty, the item is valid and the stack contains exactly one item; other stacks move between player main storage and hotbar. Native Shift-click uses these algorithms through the existing live path.

The `--only container_transfer` oracle executes real MCP919 Container.slotClick for chest, hopper, dispenser, dropper and beacon. It compares the original return stack and every resulting slot against Rust for 6,930 scenarios: every source slot, five item types, counts 1/32, and empty/full/mixed inventories. Zero differences were observed. Seven Rust transfer tests include the specialized routes. The comparison rejects incomplete, duplicate or unexpected scenarios and missing slot state; a Python integrity test covers empty, partial and duplicate input.

This probe uses deterministic storage states, without custom NBT, crafting or furnace output side effects. It does not establish complete container compatibility or exercise these new specialized routes against a live official server.

## Window property delivery and furnace progress

Protocol 47 S31PacketWindowProperty (0x31) is implemented with unsigned window ID and signed short property/value fields. The frozen registry exposes it to the driver and session forwards it through the usability path. Inventory applies properties only to the current matching container, preserves them across WindowItems refreshes and preserves newer server properties when reverting click prediction. Usability snapshots expose the properties to the native furnace overlay's fuel/cook bars. The fuel denominator fallback of 200 and integer scaling follow GuiFurnace; display widths are clamped to the available bar width. Other specialized GUI property presentations remain incomplete.

Three focused tests cover exact signed wire bytes, truncation rejection, matching window/lifecycle rules and click rollback after newer progress. The opt-in official-server test places a furnace, fills iron ore and coal using short commands, opens it through the native runtime's block interaction path and verifies positive burn/cook values and total cook time 200. It passes alongside existing mining, swaps, throws, death and respawn checks. Native progress visuals have not been manually inspected and are not yet vanilla GUI artwork.

The test's authorized /stop cleanup tolerates only ConnectionAborted/ConnectionReset socket errors caused by server shutdown; other runtime errors still fail.

## Furnace input and transfer reconstruction

`inventory/furnace.rs` stores observed numeric fuel durations and all 26 registered smelting recipes. The `--only furnace_properties` probe compares 337 registered fuel observations and 26 recipes against MCP919, with zero differences. Fuel slot validation now follows SlotFurnaceFuel: fuels and empty buckets are accepted, with an empty bucket limited to one; unrelated items are rejected. Manual input-slot placement remains unrestricted as in ContainerFurnace.

Shift transfer destination selection now follows ContainerFurnace: output goes to player storage in reverse order; other furnace slots go forward; player stacks are routed to smelting input before fuel, otherwise between main/hotbar. Empty buckets are not shift-routed as fuel. The existing container-transfer probe now includes furnace and eight item types, comparing 12,960 scenarios with zero return-stack or slot-state differences. Two new regression tests cover fuel-slot validation and smelt/fuel/ordinary routing, including noncookable fish metadata.

These changes do not implement a local furnace simulation, crafting-result side effects, achievements, or XP entity handling. The earlier official-server test validates furnace progress delivery; the new furnace transfer routes are covered by actual MCP algorithms, not yet by a live-server transfer test.

## Remote player equipment synchronization

S04PacketEntityEquipment (0x04) is implemented with VarInt entity ID, signed short equipment slot and the existing nullable/NBT-preserving Slot codec. The protocol registry delivers the packet to the runtime; the session does not treat remote equipment as local inventory.

Remote player state and movement handlers now live in client/entity/player.rs. Five equipment slots retain complete received item stacks: held item, boots, leggings, chestplate and helmet. Updates affect existing tracked players only; null removes equipment; destroy/respawn cleanup removes the tracked state. Slot 0 also updates the existing held-item ID. Invalid slot indices are ignored safely.

The equipment codec test checks golden prefix bytes, null/item roundtrip and truncated input. The entity-state test checks unknown entities, helmet/held-item changes, removal and destruction. The local TCP runtime test feeds SpawnPlayer and Equipment and verifies the helmet reaches tracked state even before the local player's initial position. Full workspace tests pass. Remote player rendering still uses debug boxes; armor models, held-item geometry and general non-player entity equipment remain incomplete.

## World time synchronization

S03PacketTimeUpdate is decoded and encoded as two signed big-endian longs and registered for live delivery. WorldSnapshot owns time state in world/time.rs. Negative world time stops daylight cycling and is negated using Java long wrapping semantics; -1 represents the server frozen-zero sentinel and becomes frozen time 1, matching WorldClient. Positive updates resume cycling. Main-thread runtime and CLI ticks increment total world age regardless of the cycle flag and advance day time only while cycling. New worlds reset time state.

One codec test verifies golden bytes and truncated input; two world tests cover frozen/running transitions, sentinel behavior and long overflow. Full workspace tests pass. The official-server probe below verifies frozen day time and advancing world age. This state does not establish visual day/night parity.

## Celestial angle and daylight rendering handoff

WorldTime exposes the local MCP919 overworld celestial-angle and moon-phase calculations. The `--only celestial` oracle compares actual WorldProviderSurface results against Rust for every tick of one day at four partial-tick offsets: 96,000 observations, with zero float-bit differences. This specifically follows the inspected local MCP source and compiled reference, not a guessed alternative smoothing formula.

render/daylight.rs computes World-style sun brightness and sky subtraction using the existing MathHelper cosine table and source float operation order. A focused test checks noon/midnight values. The native frontend now uses live world time and render interpolation to scale its existing sky background and subtract daylight darkness from mesh sky-light values, leaving block light unchanged. Dimension configurations without sky light retain the previous rendering path.

The whole workspace passes. Visual output has not been manually inspected. Current background and terrain lighting remain simplified; celestial geometry, fog, biome sky colors, vanilla lightmap and Nether/End visuals are not complete. Received weather now feeds daylight calculation. Only celestial-angle bit equality is proven by the new exhaustive oracle.

## Received rain and thunder state

WorldSnapshot now owns Weather in world/weather.rs and consumes S2B reasons 1/2/7/8. Rain-start sets the raining flag and strength zero; rain-stop clears the flag and sets strength one. Explicit strength updates replace the received value; nonfinite strengths are ignored. These states are not locally advanced because MCP919 WorldClient.updateWeather is empty. Thunder strength supplied to brightness is raw thunder multiplied by rain, matching World.getThunderStrength. New-world initialization clears weather.

Live daylight calculation now reads weather instead of fixed zeros. The native terrain sky-light subtraction consequently responds to server weather. Tests cover reason transitions, persistence across client ticks, thunder/rain multiplication, noon brightness reduction, and S2B delivery over the local TCP runtime path. Full workspace tests and the focused TCP test pass. The official-server probe below also verifies received rain state and positive strength. Rain geometry, clouds, lightning, sky-color weather desaturation and sound remain incomplete.

## Player DataWatcher metadata

S1CPacketEntityMetadata (0x1C) is registered, decoded and encoded using the existing bounded DataWatcher blob parser. codec/play/metadata.rs additionally decodes all eight types into typed values: byte, short, int, float, string, nullable item with NBT, three-int position and three-float rotation. Ordered entries are preserved, and malformed/trailing input fails.

Remote player spawn metadata initializes indexed state; subsequent metadata packets replace only mentioned indices on existing players. Entity flags read byte index 0. The native renderer suppresses invisible remote players for ordinary viewers and retains them for spectators. Team-friendly invisible visibility remains incomplete, as do sneaking pose, burning feedback and non-player metadata behavior.

Two codec tests cover packet layout, truncation/trailing failures and all eight value types. A player-state regression covers spawn flags and preservation of unrelated indices after updates. Full workspace tests pass. These new paths have not yet been compared with an authored Java metadata oracle or tested against an official server.

## Friendly invisible player visibility

UsabilityState now exposes the same-team/friendly-invisible rule from EntityPlayer.isInvisibleToPlayer and ScorePlayerTeam: scoreboard membership must match and friendly flags bit 2 must be enabled. Runtime caches the spawn profile name from the existing player-list mapping on the received player entity; later Tab-list removal does not discard that identity. Native rendering admits invisible teammates under that rule and continues to admit spectators. Team changes immediately affect the query.

A focused regression covers absent teams, same/different team, enabling/disabling the flag and removing membership. Full workspace tests pass. Friendly-invisible alpha rendering and name-tag rules remain incomplete; remote avatars are still debug geometry. Missing player-list identity at spawn also remains outside the verified path.

## Received experience and HUD state

S1FPacketSetExperience (0x1F) is implemented with float progress, VarInt level and VarInt total, matching the inspected MCP919 codec and setXPStats handler order. The protocol registry and session route it to UsabilityState. Snapshots expose all three values; native rendering draws the received progress bar and level. LiveRuntime resets experience when creating the respawned player, awaiting the server update.

A golden wire test checks field order and roundtrip. A state test verifies snapshot values and reset. Workspace tests pass, followed by the new focused state test. Real-server experience acquisition, GUI visual inspection, creative/spectator HUD visibility rules, XP orbs and enchantment interactions remain unverified or incomplete.

## Official-server experience, time and weather synchronization

The opt-in `official_server_confirms_mining_and_respawn` test now resets XP levels, grants seven levels, freezes daylight cycling, sets day time to 6000 and starts rain through commands on the isolated official 1.8.9 server. It waits for the native runtime to receive level 7, frozen day time 6000, the raining flag and positive rain strength, then verifies that total world age advances while day time stays fixed. These assertions passed alongside the existing inventory, furnace, mining, death and respawn checks. The owned test server saved its world and exited after the test requested `/stop`.

This verifies received live state, not XP orb pickup, complete weather transitions, thunder or visual HUD/sky parity. The server logged an internal StackOverflowError during player disconnect after `/stop`; the state assertions had already passed, and the process exited successfully. That shutdown log remains a limitation of the test run.

## Experience HUD source conditions and texture regions

The native frontend now gates XP rendering on survival/adventure game modes, following PlayerControllerMP and GuiIngame. Creative and spectator modes hide this XP overlay. The bar uses source Y position (height minus 29), width 182, and integer progress width from progress times 183. Capacity checks preserve EntityPlayer.xpBarCap's Java integer wrapping. The level label uses the source color, vertical position and four black outline offsets. If the user imports local GUI icons, the renderer uses the source texture regions (0,64) and (0,69); no icons are bundled or committed. Without imported icons the existing plain-color fallback remains.

A framebuffer regression verifies the five-pixel bar position and near-full progress width; workspace tests pass. This does not prove visual parity: font metrics/scaling, hotbar layout, mounted-horse jump replacement, spectator hotbar and other HUD elements remain incomplete, and imported-texture output has not been manually inspected.

## Game-state mode rounding

MCP919 NetHandlerPlayClient converts reason-3 values with MathHelper.floor_float(value + 0.5F), then GameType.getByID. Both runtime and simulation previously truncated the float directly. They now share game_mode::from_game_state, preserving Java cast saturation and wrapping floor subtraction, with unknown IDs falling back to survival. A boundary regression covers half-integer transitions, negative values and nonfinite values. This prevents movement capabilities and HUD mode from disagreeing about fractional game-state updates. Workspace tests pass; fractional values have not been exercised against an official server.

## Resource-pack request and disabled-mode response

S48 ResourcePackSend (0x48) now decodes bounded URL/hash strings (32767/40), and C19 ResourcePackStatus (0x19) encodes hash and enum ordinal (0 loaded, 1 declined, 2 failed, 3 accepted). Both directions are in the frozen protocol registry. The driver exposes the inbound request and sends exactly one DECLINED response with the received hash, following vanilla's disabled-resource-pack branch. It does not download URLs or report a successful application. Trace/report packet naming includes the status packet.

A driver regression feeds a framed request after login, checks the inbound event and exact response bytes, and rejects malformed/truncated packets and invalid status ordinals. Workspace tests pass. Only disabled-mode resource-pack behavior is implemented: consent UI, per-server preference, downloads/cache/hash checking, local level packs, actual resource reload and successful-load/failure lifecycle remain incomplete. This path has not been tested with an official server resource-pack offer.

## Server difficulty update

S41PacketServerDifficulty (0x41) is registered and decodes one unsigned byte into EnumDifficulty using modulo four, matching MCP919. Encoding writes one byte. In this reference the difficultyLocked member is not serialized; adding a boolean would incorrectly consume trailing data. WorldSnapshot now retains the received difficulty and exposes it for subsequent world/UI behavior; a new world has no received S41 value until one arrives.

A regression covers all normal IDs and byte values 4/255, exact encode/decode bytes, truncated/trailing input and world-state replacement. Workspace tests pass. Difficulty-lock UI and difficulty-dependent gameplay remain incomplete, and S41 has not yet been asserted against an official server. This addition only establishes the received update path.

## Difficulty across world lifecycle

LiveRuntime initializes difficulty from JoinGame when creating the world. On Respawn it follows the inspected NetHandlerPlayClient: same-dimension respawn preserves the existing world difficulty, while cross-dimension world creation uses the new packet's difficulty. WorldSnapshot::with_difficulty supplies this initialization without manufacturing a network update.

The framed local TCP join test asserts initial difficulty. The existing TCP death/respawn test sets difficulty to 3, receives a same-dimension respawn declaring 1 and verifies preservation of 3; its subsequent dimension-change path verifies replacement with 1. Workspace tests pass. These lifecycle assertions have not yet been repeated on an official server, and difficulty-dependent gameplay/UI remain incomplete.

## VarLong packet-buffer support

The inspected S44 WorldBorder packet needs PacketBuffer.readVarLong/writeVarLong for transition milliseconds. The Rust packet buffer now provides signed i64 VarLong operations, preserving Java long bit patterns and unsigned-shift encoding. Decoding is bounded to ten bytes, rejects an unfinished tenth byte, and retains Java's low-bit behavior for an oversized tenth payload. Failed reads do not advance the reader offset.

A regression verifies positive/negative extremes, golden negative-one and 128 encodings, truncation/oversize rejection and reading the next buffer field. Workspace tests pass. This is necessary communication support; S44 action decoding, border state/interpolation, collision and rendering still remain unimplemented. No world-border compatibility is claimed by these primitive tests.

## World-border packet action layouts

S44PacketWorldBorder (0x44) is registered and decoded into a typed action enum: set size, lerp size, set center, initialize, warning time and warning blocks. Action fields follow MCP919; transition time is signed VarLong, and initialize sends center, old/new diameter, time, world-size clamp, warning distance, warning time in that order. All actions encode as well as decode. The protocol driver now exposes these inbound packets instead of ignoring an unsupported ID.

A regression checks all six action roundtrips, the exact initialize wire bytes, each truncated prefix, trailing bytes, invalid action and overlong duration. Workspace tests pass. WorldSnapshot now applies border actions with interpolation as described below. Collision, warning overlay and boundary rendering remain required before claiming border compatibility. No official-server border assertion has been run.

## Received world-border state and interpolation

WorldSnapshot now owns WorldBorder and applies all six received actions with a wall-clock millisecond timestamp. State includes center, start/target diameter, transition timing, world-size clamp and warnings. Diameter uses Java wrapping long timing and float division before double interpolation, then settles at the target. Bounds clamp to world size, and closest distance follows the signed distance to the four sides. Initialize with nonpositive duration sets the target directly, matching S44's handler.

Two regressions cover action replacement, initialization, half-time and one-third interpolation, completion, world limits, negative outside distance, warnings and the WorldSnapshot packet path. Workspace tests pass; focused world tests were repeated after correcting Java Math.min NaN/signed-zero behavior in distance calculation. Border collision, native rendering/warning overlay, lifecycle preservation across dimensions and official-server validation remain incomplete. No full border or client compatibility is claimed.

## Player collision at received world borders

The simulation's terrain, sneak-support and step collision queries now use an entity-aware WorldSnapshot path. It follows World.getCollidingBoundingBoxes: only loaded X/Z columns contribute; blocks outside the border become full stone collision cubes while the player is considered inside. The local simulation retains the outside-border flag and applies the source one-block entry/exit hysteresis. Border collision scans use source X/Z/Y order and include negative Y for virtual border cubes. Players already outside retain ordinary terrain collisions and can re-enter.

A regression covers unloaded columns, full boundary cubes, outside-state transitions and the narrower re-entry threshold. All eight existing terrain movement tests and the whole workspace pass. Entity-vs-entity collisions, spectator no-clip, border graphics/warnings, transition timing differential probes and official-server border movement verification still remain incomplete. Earlier border-collision limitation statements above describe the state before this addition.

## Spectator no-clip movement

Local simulation now follows EntityPlayer.onUpdate's spectator on-ground clearing before movement and Entity.moveEntity's noClip branch: game mode 3 offsets position directly, bypassing terrain/border/step collisions and movement-phase web slowdown. Changing back to survival restores the ordinary entity-aware collision path. Existing spectator flight handling remains in use.

A received ChangeGameState regression crosses a solid wall at the configured world border in spectator mode, asserts no ground contact, then restores survival and verifies the wall stops motion. The first full run exposed pending authoritative state restoring on-ground after the early spectator reset; the no-clip branch now clears it again. All nine terrain tests and the corrected full workspace run pass. Complete spectator compatibility is not proven: spectator speed controls, entity camera/riding, GUI and online official-server mode-switch validation remain incomplete.
