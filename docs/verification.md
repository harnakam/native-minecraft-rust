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

## Spectator acceleration after pending server state

The pending authoritative position/ground state can overwrite the early spectator ground reset. Simulation now clears on-ground again immediately after pending state and knockback application, before movement acceleration/jump calculations. This prevents a received ground flag from selecting the ground acceleration branch for a spectator.

A regression compares identical spectator forward/jump input after authoritative states differing only in their ground flag; positions and velocities must match exactly and ground remains false. Full workspace tests pass. This strengthens the no-clip path but does not implement spectator speed scrolling, camera targets or complete spectator behavior.

## Border proximity warning handoff

WorldBorder now computes GuiIngame's warning strength: closest distance is cast to float, warning radius is the maximum of warning blocks and the smaller of time-scaled resize speed and remaining diameter change, and strength is one minus the distance/radius ratio. Runtime supplies actual simulation player position, and the native frontend applies a red edge warning before crosshair and HUD drawing. No vignette texture is bundled.

A state regression covers no warning, proximity, outside strength, resizing anticipation and completion. A framebuffer regression checks red edge tint, unchanged center and no modification at strength zero. Workspace tests pass, followed by the focused framebuffer test after the position handoff adjustment. The visible warning uses a simple edge mask, not the vanilla vignette image/blending pipeline; visual parity and official-server boundary testing remain unverified.

## Local vignette texture for border warnings

The local asset importer permits textures/misc/vignette.png and refreshes older extracted caches missing that image. GameAssets loads it locally. Border warning rendering uses its sampled green/blue channels with the GuiIngame ZERO / ONE_MINUS_SRC_COLOR destination blend; the red and alpha channels of the framebuffer remain unchanged. The previous edge fallback remains when no local image is available. No game image is tracked or bundled.

Full workspace tests pass, followed by two focused warning tests after cache/test updates. The new test loads an authored synthetic PNG and verifies exact destination pixels, texture coverage and preserved red/alpha. Actual vanilla texture output has not been manually inspected. The following update adds filtered sampling; normal brightness vignette, fancy-graphics setting and exact blending/rounding parity remain incomplete.

## Vignette blur metadata and linear repeat sampling

The local official 1.8.9 JAR vignette.png.mcmeta specifies texture blur=true. SimpleTexture passes that metadata to TextureUtil; clamp is not specified. The native border vignette now uses normalized bilinear sampling with repeated texture edges and half-texel center coordinates, preserving float channels until framebuffer blending. Other existing sprite/terrain sampling paths are unchanged.

A synthetic two-texel regression verifies exact texel centers, midpoint interpolation and repeating edge interpolation. The existing loaded-PNG warning framebuffer regression and the whole workspace pass. This confirms the CPU sampling path, not GPU pixel-for-pixel visual parity; normal light-level vignette and complete graphics-settings behavior remain incomplete. No vanilla image or metadata is published.

## Official-server border-state delivery

The opt-in official 1.8.9 probe now issues worldborder center/set/warning commands and asserts received center (4.5,-3.5), diameter 64, warning distance 7 and warning time 11. CommandWorldBorder parses integer center arguments with block-center adjustment, so the original expected (4,-4) assertion failed despite correctly received state; the source-confirmed expectation was corrected. The rerun passed with the inventory, mining, experience/time/weather and death/respawn checks. The first run demonstrated changed state delivery; the rerun reused those center/warning values. This does not independently prove every action changed state on the second run.

The test restores diameter 60000000 and issues center 0 0 (which sets 0.5,0.5), then requests server stop. Server logs recorded world saves and the owned process exited with code zero; console logging emitted an exception during shutdown. Workspace tests pass. Border movement collision, resizing interpolation and visual warning parity still require official-server/differential validation.

## Local Java world-border diameter oracle

The authored RmcBorderOracle instantiates actual compiled MCP919 WorldBorder, starts a transition and sets its private timing fields to controlled origins/durations. It captures wall-clock timestamps immediately before and after getDiameter. The Rust border_oracle_check example requires the Java result's raw double bits to match a Rust diameter at an integer millisecond within that captured window. This accounts for the unmocked System.currentTimeMillis call without substituting a copied Java formula.

Run `python scripts/verify_border_oracle.py --java-home <Java8 home> --cargo <cargo path>`. All 189 observations passed: three starting diameters, three targets, three durations and seven elapsed offsets, covering growth, shrinkage and completion. Timing windows must be ordered and under one second. This is a clock-window comparison, not an exact fixed-time oracle or exhaustive input proof. Bounds, collision, warning strength and native visuals are outside this probe. Only authored harness/checker code is tracked; compiled reference classes remain local and ignored.

## Exact stationary border bounds and distance oracle

The authored border oracle now also calls actual MCP919 minX/maxX/minZ/maxZ/getClosestDistance on stationary borders, avoiding clock ambiguity for these values. Five center positions, two world-size limits, four diameters and three query positions produce 120 scenarios and 600 double-bit comparisons. All 600 match Rust exactly, including world-limit clamps, degenerate zero diameter and negative outside distance. The existing 189 transition observations were rerun successfully in the same probe.

The same verify_border_oracle.py command runs both groups. This establishes equality only for the listed finite stationary scenarios; moving bounds, nonfinite/signed-zero corner cases, full collision lists, warning strength and visual output remain outside this comparison.

## Spectator wheel flight-speed control

The native wheel handler now routes spectator scroll to the runtime, shell and simulation instead of hotbar selection. Simulation follows Minecraft.runTick's closed spectator-menu branch: normalize event direction, add direction times 0.005F to fly speed and clamp to 0..0.2. This is a local capability adjustment and does not manufacture a PlayerAbilities packet. The existing fly-speed-dependent acceleration uses the changed value.

A regression verifies non-spectator rejection, sign normalization, increased actual forward motion, both speed limits and no unsolicited ability packet. Full workspace tests pass. Spectator GUI/menu selection, GUI-open input gating and official-client input/flight differential validation remain incomplete; this only implements the closed-menu speed behavior.

## GUI gating for wheel gameplay input

The native wheel event handler now checks the same blocking states as frame-input consumption before directly adjusting spectator fly speed: chat open, mouse released, dead player or open container block gameplay scrolling. This closes the bypass introduced when spectator speed updates moved directly into the event handler. Normal captured gameplay retains ordinary hotbar scrolling or spectator speed adjustment. The source Minecraft.runTick screen/allowUserInput gate was inspected; native chat/container overlays currently have no allowUserInput exception.

Full workspace tests pass. The simple event guard was reviewed against existing overlay/cursor state transitions, but native interactive GUI wheel behavior has not been manually exercised. Chat scrollback and spectator GUI selection remain incomplete; this change prevents gameplay-side mutations while those native overlay states are active.

## Native received-chat history scrolling

While chat is open, native rendering shows up to eight retained received messages and wheel input changes the history offset instead of gameplay state. It follows GuiChat's normalized direction, seven-line step and Shift single-line step, clamps to retained history, and resets offset on opening chat. Text/string/array/extra components are flattened, with standard chat.type.text and chat.type.announcement formatting; unknown translation keys expose the key and arguments rather than dropping the message.

Workspace tests pass, followed by a focused component regression checking player-name/message formatting, nested extras and arrays. Interactive scrolling has not been manually exercised. The current 64-message retention, fixed row count, truncation rather than wrapping, translation coverage, style/click/hover/font handling, new-message scroll anchoring and closed-chat fading remain incomplete; this is a working received-history path, not complete GuiNewChat parity.

## Wrapped native chat display rows

Native history now wraps text to the current framebuffer chat width instead of truncating messages. It prefers word boundaries, preserves explicit newlines and splits long unbroken text by Unicode characters. Wheel limits and drawing both use the resulting display rows, retaining the newest 100 rows. The underlying received-message retention increases from 64 to the source GuiNewChat's 100 messages. Position-2 action-bar messages are excluded from history.

A regression covers word wrapping, explicit newlines, unbroken ASCII/Unicode and display-row expansion at a narrow width. Full workspace tests pass after correcting the edited file's Windows encoding to UTF-8. Complete font/formatting widths, Unicode glyph rendering, action-bar display, layout settings and new-message scroll anchoring remain incomplete; this removes native truncation but does not prove FontRenderer/GuiNewChat layout parity.

## Long-chat wrapping cost

Native wrapping no longer drains and shifts the remaining character vector on every display row. It keeps a read offset into an immutable character vector, preserving existing wrap decisions while avoiding quadratic data movement for narrow chat layouts and long unbroken server messages. A regression wraps 32767 characters at one column and verifies complete reconstruction. Existing wrap/component regressions and full workspace tests pass. Chat layout is still rebuilt for drawing; caching, source font-width parity and interactive performance profiling remain incomplete.

## Chat keyboard paging

Open native chat now handles PageUp/PageDown using GuiChat's visible-line-count-minus-one movement: seven rows for the current eight-row display. Wheel and keyboard input share one bounded scroll method, with saturation before clamping to retained wrapped display rows. Keys are handled only in the open-chat branch and do not enter movement input.

Full workspace tests pass, and key routing was reviewed against the existing pressed/released handling. Native interactive paging has not been manually exercised. Configurable visible row count, sent-input history, caret editing, tab completion and full native keyboard parity remain incomplete.

## Sent chat input history

Native open-chat Up/Down keys now traverse submitted messages following GuiChat.getSentHistory: clamp to history bounds, save the current draft on first leaving the newest position, and restore it on returning. Opening chat resets the cursor. Successful runtime submission records the trimmed message; adjacent equal messages are deduplicated like GuiNewChat.addToSentMessages. Failed submission preserves input and does not record a new entry, and absent runtime now reports unavailable rather than clearing the draft as though sent. History is in-memory for the app session.

A regression checks adjacent deduplication, oldest-bound clamping, traversal and draft restoration. Full workspace tests pass, followed by the focused history test after the absent-runtime guard. Server acceptance is not implied by successful local submission. Interactive keyboard verification, caret/selection editing, completion and complete chat formatting remain incomplete.

## Chat input filtering and Java-length limit

Native typing and clipboard insertion now share a filter based on ChatAllowedCharacters: reject characters below U+0020, DEL and section sign. GuiChat's 100-character input limit is measured in UTF-16 code units, so supplementary characters consume two units rather than one Rust scalar. Insertion stops before exceeding the limit and keeps input valid UTF-8. Launcher text fields retain their existing separate filtering.

A regression checks filtered pasted controls/section sign, exact UTF-16 limits, supplementary-character boundaries and subsequent insertion. Full workspace tests pass. Rust does not retain isolated surrogate halves when a supplementary character would straddle the limit, unlike Java substring behavior; caret/selection-aware insertion, native clipboard interaction and complete Unicode font behavior remain incomplete.

## Protocol string UTF-16 bounds

PacketReader and PacketWriter now measure Minecraft string length in UTF-16 code units, matching PacketBuffer's Java String.length, instead of Rust Unicode scalar count. This applies to bounded strings throughout the codec. ChatMessageServerboundPacket::vanilla also truncates by the 100-unit limit, so programmatic/runtime submissions bypassing native input do not send more supplementary characters than Java accepts.

A regression checks exact supplementary-character UTF-8 bytes, one-unit rejection/two-unit acceptance in both directions, 50-emoji chat construction/roundtrip and over-limit direct packet rejection. Full workspace tests pass. Invalid UTF-8 replacement behavior and isolated-surrogate truncation remain outside proven parity; Rust keeps complete valid scalars when truncation would split a surrogate pair. No complete text-protocol/client parity is claimed.

## Java surrogate-boundary chat wire replacement

A local Java 8 probe confirmed that C01-style substring(0,100) of 99 ASCII characters followed by a supplementary character leaves an isolated high surrogate; PacketBuffer.writeString's String.getBytes(UTF_8) encodes it as ASCII question mark (byte 63), yielding 100 message bytes. ChatMessageServerboundPacket::vanilla now produces that same wire replacement rather than dropping the partial character entirely. Rust stores the replacement question mark, not Java's isolated-surrogate intermediate String.

A regression verifies exact packet ID, length prefix, total bytes and final replacement byte. Full workspace tests pass. Native input still avoids isolated-surrogate insertion at its own GUI limit, and general malformed UTF-8/UTF-16 behavior remains outside proven parity. The temporary Java probe is local and ignored; no reference code or assets are committed.


### Java UTF-8 replacement decoding

MCP919 `PacketBuffer.readStringFromBuffer` constructs a Java String with UTF-8,
which replaces malformed input rather than reporting a decoding error. An
independently authored local Java 8 probe checked ten byte sequences: surrogate
encodings and incomplete prefixes, overlong three/four-byte encodings,
out-of-range code points, interrupted sequences, and illegal two-byte leaders.
The Rust packet reader now reproduces those observed replacement strings,
including Java's single replacement for an encoded surrogate. The regression
also checks UTF-16 limits after replacement and rejects truncated packet bodies.
NBT's separate string decoder is unchanged. This is a bounded set of Java
observations, not exhaustive evidence for all malformed byte streams or complete
client compatibility. The probe and Java binaries remain ignored local files;
no Minecraft assets or reference source are included.


### Native closed-chat display and tick fade

Incoming chat lines now retain an age in simulation ticks. LiveRuntime advances
that age with the actual scheduled tick count, and wrapped rows inherit the
message age. The native HUD draws chat with the input screen closed, using the
MCP919 GuiNewChat alpha formula: full strength through approximately tick 180,
quadratic fade through tick 199, hidden at tick 200. Opening chat overrides the
fade without discarding history. Hidden chat visibility suppresses drawing.
Background alpha is half of text alpha and is blended into the rendered frame.
Regression tests cover message age, wrapped-row age, the alpha boundaries, and
actual framebuffer changes for fresh/faded/open/expired lines.

Remaining differences include native font metrics, row geometry, configurable
opacity/scale/height, formatting and interactions, and
scroll anchoring for incoming messages. A scheduled frame may apply a batch of
ticks to a newly received message; per-tick packet dispatch timing is not yet
matched. No manual window or official-client visual comparison was performed.


### Native S02 action-bar notifications

MCP919 NetHandlerPlayClient routes S02 position 2 to GuiIngame's overlay message
rather than GuiNewChat. UsabilityState now stores a separate latest message,
replaces it on receipt, and expires it after 60 scheduled ticks. These packets no
longer consume chat-history capacity. Native drawing uses centered unformatted
component text at the source HUD offset and the source linear alpha formula,
including the existing render interpolation fraction; ordinary chat visibility
does not suppress this separate overlay. It shares the actual text blending path
with chat history. Tests verify replacement, expiry, exclusion from chat history,
partial-tick alpha, and framebuffer effects without a background rectangle.
The whole workspace test suite passes. Official-server position-2 delivery and
manual visual parity have not been verified. Native fonts, complete translation,
record-playing color cycling, mounting messages, and exact per-tick network
dispatch remain incomplete.


### S45 title notifications

Added the protocol-47 title packet (0x45) to the frozen registry and driver decode
path. All five action layouts are supported: title/subtitle JSON components,
three signed big-endian Int timing fields, clear, and reset. Codec regressions
cover exact timing bytes, all action round trips, truncated bodies, unknown
actions, and trailing bytes. TitleState follows MCP919 GuiIngame.displayTitle and
NetHandlerPlayClient.handleTitle: subtitle alone does not start a timer; title
starts the combined duration; nonnegative time updates preserve negative fields
and restart an active timer. Java int duration arithmetic is retained.

The source RESET handler calls displayTitle with two non-null empty strings,
whose title branch clears only the title and restarts the old duration, then
restores default times (10/70/20). The existing subtitle survives that call; this
source behavior is explicitly tested rather than substituted with a generic
clear. CLEAR and all-negative TIMES clear both lines and stop the timer.
Native title/subtitle rendering is connected to the usability snapshot, with
4x/2x centered text and source fade alpha including interpolation. A regression
feeds encoded/decoded title packets through usability state into the actual
framebuffer and verifies CLEAR removes drawing. Whole-workspace tests pass.

Remaining gaps: official-server title command delivery, manual visual parity,
Minecraft font metrics and shadows/styles, complete component translation, and
exact tick-order network dispatch. No Minecraft assets or reference source are
published.


### Official title delivery and session routing repair

The first official-server test exposed a missing connection in the previous
S45 change: the driver decoded the packet, but HeadlessSession discarded its HUD
routing action. Direct usability/framebuffer tests had not covered that boundary.
Title packets now produce SessionAction::UsabilityPacket, reaching LiveRuntime's
existing HUD update path. A regression covers this routing for all five actions.

The isolated official 1.8.9 server at 127.0.0.1:25570 then passed the expanded
ignored integration test. Through actual /title commands and the compressed
connection, it verifies 4/30/6 timing configuration, subtitle receipt without
starting a timer, title plus retained subtitle with an active timer, active
time replacement to 2/50/8, CLEAR of both text fields and timer, and RESET of
timing defaults. The test also passed its existing inventory, furnace, XP,
time/weather, border, mining, death, and respawn checks. The same live server was
reused after the failed attempt; no restart was used to mask the routing failure.
The test issued /stop, the server saved players/worlds/chunks, and the owned
server process exited successfully. Title state reached the runtime in real
communication; manual rendering parity and exact visual fade remain unverified.


### S47 tab-list header/footer

Added S47PacketPlayerListHeaderFooter (0x47), preserving header-then-footer Chat
field order, to the codec and protocol registry. It is explicitly forwarded by
HeadlessSession to the HUD state. Usability snapshots retain the latest pair,
replacing both fields for every packet. Empty rendered component text hides the
corresponding banner. The native Tab overlay draws wrapped centered header lines
above the player list and footer lines below it, including header/footer-only
snapshots. The overlay remains controlled by the existing Tab key.

Codec tests cover round trip, truncated fields, and trailing bytes. An integrated
regression decodes real packet bytes, applies the session routing action and HUD
state, draws into a framebuffer, and verifies that a subsequent empty pair
removes the banner pixels. It also covers explicit newlines. Publication contains
only independently authored code, with no reference assets or source.

Remaining gaps include Minecraft font/layout/style parity, complete translation,
80-player multi-column layout and sorting, player faces, ping icons, scoreboard
columns, reset timing across connection transitions, and actual server/plugin
S47 delivery. This change does not establish full Tab-screen compatibility.


### Native tab ordering and 80-player columns

UsabilitySnapshot now sorts tab entries by non-spectator first, registered team
name, then profile name, matching GuiPlayerTabOverlay.PlayerComparator. String
comparisons use Java UTF-16 ordering; latency no longer affects the order.
The native overlay displays up to 80 players, increasing the column count until
ceil(count/columns) is at most 20, with column-major placement and five-pixel
column gaps. Server display-name components replace profile names when present.
Cells clip their own rendering to avoid writing into another column; spectator
text is dimmed. Header/footer layout remains connected above/below the grid.

Tests cover column thresholds from 0 through more than 80 players, spectator and
team ordering, and framebuffer evidence that player 80 is drawn while player 90
is excluded. Minecraft metrics, styled team display names, italics, skin faces,
ping icons, score/heart columns, and manual/server 80-player display parity remain
incomplete. The native font uses 16-pixel rows, so geometry is not claimed to be
identical to the nine-pixel Minecraft font.


### Native tab ping atlas rendering

The Tab overlay now receives the already locally imported icons texture and
renders a 10x8 ping sprite at cell-width minus 11. The MCP919 thresholds are
negative=atlas row 5, <150=0, <300=1, <600=2, <1000=3, otherwise=4; texture origin
is (0,176+row*8). Missing local textures retain numeric latency display.
A generated synthetic PNG regression checks all six atlas rows and framebuffer
sampling; boundary tests cover the transition values. All Tab regressions and
the whole workspace suite pass. No Minecraft image is committed or embedded.
Actual imported-icon visual parity and manual/native window observation remain
unverified; this does not complete the Tab HUD's remaining font, faces, style,
and scoreboard behavior.


### Team-decorated native tab names

Tab snapshots now carry the latest team prefix + profile name + suffix fallback
from the received scoreboard team state. The native list uses that fallback
unless a server display-name component is present, matching the priority in
GuiPlayerTabOverlay.getPlayerName. Team changes now mark both scoreboard and tab
state as updated. Tests cover team creation, prefix replacement, removal from a
team, actual framebuffer changes, and server display-name override priority.
Full-workspace tests passed; follow-up targeted tests cover the update flag.
Legacy formatting codes/styles, Minecraft typography, server-side live team
visual comparison, and complete Tab-screen compatibility remain unverified or
incomplete. No Minecraft assets or reference source are included.


### Native integer Tab scores

Tab snapshots resolve display slot 0, objective render type, and each player's
profile-name score from received scoreboard state. Missing scores resolve to
zero as in Minecraft's score lookup. Native cells display integer objectives in
yellow, reserve space before the ping indicator, and omit scores for spectators.
Tests cover display slot selection, score updates (including a negative value),
objective removal, actual framebuffer changes, and spectator omission.
The workspace suite and the final targeted state test pass. Heart-rendered
objectives are retained in state but their animated heart rendering is not yet
implemented. Exact width/layout, live official-server score delivery, fonts,
and manual display parity remain incomplete. No game assets are published.


### Official Tab score and team synchronization

The expanded isolated official 1.8.9 integration test now creates a dummy
objective, assigns it to the list display slot, and sets VanillaProbe's score.
It verifies runtime snapshots receive 42 and then -7 through the actual server
connection. It creates a team, joins the player, and applies the red color;
the expected source-formatted name is section-sign c + VanillaProbe +
section-sign r. Leaving the team restores the profile name, and removing the
objective clears the Tab score. Setup removes names from any interrupted run;
cleanup removes the created objective and team.
The ignored test passed together with its inventory, furnace, XP/time/weather,
border/title, mining, death, and respawn checks. It issued /stop and the owned
server process saved its state and exited successfully. This proves runtime
synchronization for these commands; native visual style/font comparison,
heart-rendered scores, and server/plugin S47 header/footer delivery remain
unverified. Reference binaries/worlds stay in ignored local directories.


### Legacy team color rendering in native Tab names

The native Tab-name renderer now interprets section-sign color codes with the
MCP919 FontRenderer RGB palette, including the gold exception and case-insensitive
codes. Reset restores the original row color. Formatting pairs do not consume
the visible-name truncation limit. The actual cell renderer draws each colored
run instead of showing the literal section-sign characters.
Tests verify red/reset, gold, uppercase green, visible truncation, and red pixels
in the framebuffer. The complete workspace suite passed. Bold, italic,
underline, strikethrough, obfuscation, anaglyph palette, full JSON style handling,
Minecraft font metrics and manual visual parity remain incomplete. These style
codes are consumed without their visual effects in the current Tab-name path;
this is explicitly partial formatting support, not full styled-text parity.


### Native Tab bold and line decorations

Tab-name formatting now tracks bold (section-sign l), strikethrough (m), and
underline (n), accumulating them across runs. Color changes and reset clear
these flags, matching the FontRenderer control flow. The cell renderer draws
bold with a second glyph at x+1 and advances by an extra pixel; underline and
strike draw actual horizontal lines. Tests check flag accumulation/reset, bold
framebuffer changes, and pixels on both line positions. Whole-workspace tests
passed. Native font metrics and line positions use the current 16-pixel native
font layout; they are not an exact reproduction of Minecraft's font. Italic,
obfuscated text, shadows, styled JSON components, and manual visual parity remain
incomplete. No game assets are added to publication.


### Native Tab italic and spectator names

The Tab-name style parser now tracks section-sign o and clears italic on color
changes and reset. Spectator rows prepend italic to the resolved display name,
as GuiPlayerTabOverlay does; embedded color codes can subsequently clear it.
Italic glyph coverage is sheared in the cell framebuffer and blended with the
selected text color, including bold overlap. Tests check italic/reset ordering
and actual glyph pixel changes, and the complete workspace suite passes.
This uses the current native 16-pixel glyph rasterization and a per-row shear;
Minecraft atlas glyph geometry, shadows, spectator alpha, obfuscation, complete
JSON style handling, and manual visual parity remain incomplete. No Minecraft
image/source data is published.


### Spectator Tab name opacity

MCP919's spectator name color -1862270977 is ARGB 0x90FFFFFF. The native Tab cell
now uses white for ordinary names and alpha 144 for spectator names instead of
an opaque gray approximation. Formatting colors retain this opacity. The
rendered glyph result is blended with the cell's prior RGB, preserving framebuffer
alpha. Tests cover colored/italic names against a nonblack background, alpha-zero
identity, and the alpha-144 result. The whole workspace suite passes.
This currently composites the completed name layer; per-glyph overlap blending,
OpenGL rounding, text shadows, Minecraft glyphs and manual visual comparison
remain unverified or incomplete. This is not full rasterization parity.


### Styled JSON Tab display names

The native Tab display-name path now converts text components and extra children
into formatted text for its existing style renderer. Named colors, bold, italic,
underline and strike inherit from the parent; explicit boolean false clears the
corresponding flag. Each text segment ends with reset, and uncolored segments do
not introduce a white color code that would unnecessarily cancel spectator
italic. Tests cover nested inheritance, explicit false, color override and actual
colored pixels. The workspace tests passed before the reset-order adjustment;
all ten Tab tests passed after it. Translation arguments still flatten through
the existing partial translation helper; obfuscation, full component semantics,
array-root inheritance, click/hover events, Minecraft fonts and manual rendering
parity remain incomplete. No game assets/reference source are published.


### Array-root Tab component style inheritance

MCP919 IChatComponent.Serializer makes the first array element the root and
appends later elements as siblings; ChatComponentStyle.appendSibling sets each
sibling's style parent to that root. The native Tab formatter now returns each
component's resolved root style and applies it to array siblings. A sibling's
own color/boolean overrides do not leak into the next sibling. Nested arrays
inherit the outer root and apply their own first-element style to their siblings.
Regression tests cover red/bold root inheritance, green/nonbold sibling override,
restoration for a later sibling, and nested italic inheritance. The actual Tab
path uses this formatter, and whole-workspace tests pass. Empty/invalid component
validation, translations, obfuscation, events, Minecraft fonts and manual visual
parity remain incomplete; this does not prove full chat-component compatibility.


### Primitive JSON Tab components

MCP919 IChatComponent.Serializer converts every JSON primitive with getAsString,
and object text properties likewise accept primitive values. The native Tab
formatter now renders integer/boolean primitives and primitive text properties
instead of dropping them, including inherited color for primitive extra children.
Regression tests verify positive/negative integers, both booleans, text property
values, child inheritance, and actual framebuffer pixels. Workspace tests pass.
Numeric lexical fidelity is still incomplete: serde_json's parsed number string
may normalize exponent/decimal spelling whereas Gson preserves input spelling.
Invalid/null components, complete translations, obfuscation, Minecraft fonts,
and manual visual parity remain incomplete. No game assets/source are published.


### Gson numeric precision observations

An independently authored local Java probe using the MCP919 Gson 2.2.4 jar
observed getAsString results for 1e+03, 1.2300, -0, 0.0000, and a 30-digit integer.
The first Rust regression failed because 1e+03 became 1000.0. Enabling the existing
serde_json arbitrary_precision feature retains the tested exponent spelling,
trailing decimal zeros, and large integer precision; -0 becomes 0 as observed in
Gson. No dependency/version or lockfile change is required. The feature applies
to the client crate's serde_json use, with the workspace suite used as regression
coverage.
This still does not preserve every numeric lexeme: serde_json's local exponent
scanner lowercases E and inserts a plus when absent. A raw-token component parser
is still needed for exact Gson lexical parity. The Java probe/jar remain local
ignored files; no reference binaries or game assets are published.


### Raw numeric tokens in native Tab components

A second local Gson 2.2.4 probe confirmed exact text for 1E3, 1e3, 1e-03, -0
(normalized to 0), and -0.00. Tab display-name parsing now uses serde_json's
existing raw_value feature to traverse raw component tokens, converting numbers
to display strings without exponent normalization. Integer tokens fitting Java's
long range use decimal conversion; other valid tokens retain their original
spelling. The ordinary serde JSON validation and depth bound run before raw
traversal. Nested text/extra values preserve their spelling and color inheritance.
Regressions cover the observed Gson cases, nested exponents/decimal zeros,
malformed numeric syntax, and excessive nesting. No dependency version or
lockfile change is required.
This path is currently wired to Tab display names; other component consumers
still use their prior parsers. Full Gson lenient syntax, invalid component
semantics, translations, obfuscation, fonts and manual parity remain incomplete.
The independently authored Java probe and reference jars remain ignored locally.


### Shared numeric component parsing across native HUD consumers

The raw-token parser is now named parse_chat_component and is used by incoming
chat rows, Tab banners/display names, action-bar text, and title/subtitle text.
The plain-text component flattener accepts boolean and numeric primitives and
primitive text properties instead of silently dropping them. A regression
checks an exponent, boolean child and trailing decimal zeros through actual chat
row/banner helpers. The workspace suite passes; a final client test rerun covers
the title call-site adjustment. Full formatted chat/title rendering, translations,
lenient Gson syntax, component validation, font and manual visual parity remain
incomplete. No game assets/reference source are published.


### Styled native title/subtitle rendering

Title/subtitle JSON now passes through the existing inherited-style formatter
instead of losing its style in plain-text flattening. The native renderer measures
styled runs, draws their coverage at the 4x/2x title scales, blends each run's
color with the existing title fade alpha, and supports bold/italic/underline/strike.
A framebuffer regression verifies red/bold JSON affects the scaled title pixels;
the existing title wire/clear test and the complete workspace suite pass.
This reuses the native 16-pixel font rasterization at integer scale, so Minecraft
font metrics, shadows, exact glyph geometry, translation styles, obfuscation,
OpenGL rounding and manual visual parity remain incomplete. No game assets are
published.


### Styled native title shadows

The title renderer now follows FontRenderer's shadow-first then foreground
ordering. Shadow offset is one source pixel in each axis, multiplied by the
4x/2x title scale. Shadow colors use the source quarter-bright palette, with the
gold shadow exception (42,42,0), and share title fade alpha. A regression checks
palette colors and darker shadow pixels outside the foreground glyph footprint.
The complete workspace suite passes. Native glyphs, rasterization/rounding,
obfuscation, translations and manual visual parity remain incomplete; no game
assets/reference source are published.


### Server-valued score chat components

MCP919 ChatComponentScore retains its explicit value; dynamic score resolution
requires the in-process MinecraftServer. The native multiplayer HUD now renders
score.value through both plain and styled component paths, preserves extra text
and inherited styling, and leaves absent values empty. This is connected to chat,
Tab names, title/subtitle and other existing component consumers. A regression
verifies value + extras, red/bold inheritance, colored title framebuffer output,
and absence of a value. The workspace suite passes.
Integrated-server resolution, required name/objective validation, translations,
fonts and manual visual parity remain incomplete. Official-server delivery is
verified below. No reference source, binaries or game assets are published.

### Official tellraw score-component delivery

The isolated official 1.8.9 test sends /tellraw VanillaProbe with a score component
referencing native_tab, after setting that objective's score to 42. It verifies
received chat JSON contains that objective and explicit score.value "42",
confirming server resolution and the compressed S02 receive/session/HUD path.
The expanded ignored test passed together with existing inventory, furnace,
experience/time/weather, border/title, Tab/team, mining, death and respawn checks.
It requested /stop; the owned server saved players/worlds/chunks and exited
successfully. Native display parsing/rendering has separate regressions; this
real-server test verifies delivery to runtime state, not manual visual parity.
No official binary/world/reference source is published.


### Unresolved selector component text

MCP919 ChatComponentSelector.getUnformattedTextForChat returns its literal
selector pattern. The native plain and formatted component paths now preserve
that pattern and inherited styles/extra siblings instead of displaying nothing.
A regression covers a radius selector with red/bold extra text and score-before-
selector precedence. This does not implement client-side entity selection;
server command resolution and invalid-component validation remain separate work.


### Component kind precedence

MCP919 IChatComponent.Serializer chooses text before translate, score and
selector, including an empty text string. The plain native path previously
overrode text with a translation when both keys were present. A regression
failed with "<Alex> ignored!" instead of "literal!" before the fix and now covers
both nonempty and empty text, extra siblings, and the formatted path.


### Translation placeholder argument count

MCP919 StringTranslate falls back to the key itself and ChatComponentTranslation
only emits arguments referenced by format placeholders; missing positive-index
arguments are skipped. Native unknown keys without percent formats now display
the key without appended arguments or a trailing space. The two supported chat
formats ignore extra arguments and omit missing argument text while preserving
literal brackets/spaces. Regressions cover both display paths and extra siblings;
the unknown-key case failed before the fix. Full language asset lookup, arbitrary
percent formats and translated argument styles remain incomplete.


### Translation percent formatting

A shared native formatter now expands sequential %s, indexed %N$s and literal
%% in translation text, including unknown-key fallback strings. Indexed
arguments do not advance the sequential counter; absent positive arguments
emit no text. The two built-in chat formats use the same formatter. Tests cover
mixed ordering, repeated indices, missing arguments, literal percent and malformed
formats (unsupported conversion, dangling percent, zero/overflow indices).
MCP919 ChatComponentTranslation.initializeFromFormat is the reference. Invalid
formats currently fall back to the key in the native display path rather than
propagating the Java component exception; language lookup and argument style
inheritance are still incomplete. No language assets or reference code are shipped.


### Styled translation arguments

The shared formatter now retains literal spans and argument components instead
of flattening them before formatted display. Literal spans inherit the translation
style; argument components resolve their own overrides against that parent, and
extra siblings retain the parent style. A regression covers a red/bold parent,
blue nonbold player name, italic message and plain inherited punctuation/extra.
This follows MCP919 ChatComponentTranslation.getFormatArgumentAsComponent and
setChatStyle. The shared parts parser also drives plain text output. Full language
lookup, invalid component exceptions and manual font parity remain incomplete.


### Executed MCP919 translation formatter comparison

A local Java 8 probe subclass called the compiled MCP919
ChatComponentTranslation.initializeFromFormat and read its actual child text.
Thirteen cases confirmed mixed indexed/sequential ordering, missing arguments,
escaped percent, leading-zero indices, maximum signed index and adjacent
placeholders. The Rust regression now includes those boundary outputs. The
reference rejects unsupported/dangling/zero/overflow/malformed formats; zero
and overflow throw distinct Java runtime exceptions, while Rust represents all
formatter failures as None. Native display still falls back to the key on error
and therefore does not reproduce exception propagation. Probe source/classes
and logs are ignored local verification files and are not published.


### Local English language asset loading

GameAssets imports en_US.lang from the user's local 1.8.9 jar into ignored
local_assets; the extraction cache now also requires that language file. Native
app initialization installs the loaded table once, and both plain/formatted
translation paths consult it before their built-in fallback formats. Read failures
are surfaced as an asset notice. Synthetic tests cover comments, first equals
separator, CRLF, empty values, duplicate replacement, path restrictions, and
component arguments retained through loaded formats. No language text is shipped.
This is startup English lookup only: language switching, resource pack overlays,
MCP numeric-format normalization and live reload remain incomplete. No manual
native window/language rendering verification is claimed.


### Language numeric format normalization

The local language loader now applies MCP919 StringTranslate's numeric format
normalization: lowercase d/f conversions with optional indexed digits and
digit/dot width fields become %s, retaining the optional index. No new dependency
is added. A regression failed before implementation and now covers widths,
precision, leading-zero/oversized indices, percent adjacency, unchanged unsupported
flags/case and UTF-8 surrounding text. A Java 8 probe reads the compiled MCP919
private numericVariablePattern and confirms five grouped fixture outputs.
This normalization runs before the existing plain/styled translation formatter.
Language switching, pack overlays, reload and invalid-format exception propagation
remain incomplete; assets/probe classes are not published.


### Actual local-jar language integration

An isolated ignored test calls GameAssets.load against the local official 1.8.9
jar, verifies an English table with more than 1,000 entries, installs that table
through the native startup lookup, and formats multiplayer.player.joined in
plain and styled paths. It verifies the loaded format replaces the key, includes
the player name, and retains blue argument/yellow parent styles. The dedicated
test passed (1 test, 0 failures); it restores its temporary working directory
and must run alone because the startup table is process-global. Imported files
remain in ignored local_assets. This proves real language import/display data
integration, not manual window/font parity or server delivery of this message.


### Selected score component validation

The shared display parser now rejects selected score components lacking name or
objective, nonobject score payloads, and null/object/array values for those fields
or optional value. Gson JsonUtils.getString accepts all JSON primitives, so
boolean/numeric fields remain accepted. Validation follows text/translate
precedence and recurses into component arrays, selected translation arguments
and extra siblings. A regression failed on an empty score object before the fix.
This is display-parser validation only: protocol ingress rejection, validation
of other component kinds/styles and vanilla exception/disconnect behavior are
not yet reproduced.


### Component container and selector structure

The shared native display parser now rejects empty/nonarray extra, selected
nonprimitive selectors, objects without a component kind, nonarray with on
selected translations, and null child components. Lower-priority unused fields
remain ignored. A regression failed on empty extra before implementation.
An isolated Java 8 probe calls compiled MCP919 IChatComponent.Serializer
jsonToComponent for nine fixtures: seven rejection and two acceptance decisions
match Rust, including a primitive boolean selector and ignored malformed fields
on a text component. Exception classes differ and ingress/disconnect handling
remains incomplete. Text/translate value coercion, styles and empty component
arrays still require further comparison. Probe files and reference assets are
ignored and are not published. The full workspace suite passes.


### Gson text and translation string coercion

Selected text/translate values now use Gson-compatible primitive or recursively
single-element-array coercion before native display. Empty/multiple arrays,
objects and null are rejected; lower-priority translate remains ignored when
text is selected. Numeric token spelling is retained by coercing after raw-token
conversion. Native plain/styled tests cover nested numeric arrays, boolean keys
and a singleton chat translation key. Executed MCP919 serializer/plain-text
comparison confirms eight acceptance/rejection and output cases. Full workspace
tests pass; error types and network ingress behavior remain incomplete.


### Gson style boolean coercion

The display parser now normalizes bold/italic/underlined/strikethrough/obfuscated
fields using Gson-style primitive or singleton-array string coercion followed by
case-insensitive true comparison. Explicit false values therefore override
inherited style even when encoded as strings or numbers; malformed null, array
and object fields are rejected. Executed MCP919 serializer iteration confirms
parent A flags true/true/false/false, child B false/false/true/true and three
invalid-field rejections, matching native regression results. Workspace tests
pass. Obfuscated flag coercion is supported, but animated obfuscated glyph
rendering remains incomplete, as do exact exception/ingress behavior and fonts.


### Component reset color inheritance

Formatted component traversal now retains the actual inherited color code,
including reset, instead of deriving it only from RGB. color reset clears parent
color while inherited bold/italic flags are reapplied, and following siblings
retain their parent color. An executed MCP919 getFormattedText probe outputs
&c&lA&r&r&lB&r&c&lC&r (ampersand standing for the section sign); the native
regression matches this exact sequence and verifies reset honors a nonwhite
draw base color. The workspace suite passes. Font/other enum color formatting
values, obfuscation and manual visual parity remain incomplete.


### Formatting enum names in component color

MCP919 deserializes color through the full EnumChatFormatting enum, so bold,
strikethrough, underline, italic and obfuscated are accepted in that field.
Native formatted traversal now retains their corresponding l/m/n/o/k codes,
including replacement of inherited color and restoration on following siblings.
Executed MCP919 getFormattedText outputs match the five exact native regression
sequences. A color bold code still produces bold glyph runs even if the separate
bold flag is false, matching prefix semantics. Full workspace tests pass.
Animated obfuscated glyph rendering, invalid enum input conversion and manual
font parity remain incomplete. No assets/reference classes are published.


### Obfuscated flag propagation and code ordering

Native component traversal and legacy styled runs now retain the obfuscated
flag. It inherits to extra siblings, supports explicit false overrides, and is
cleared by color/reset codes. Formatted prefixes emit flags in MCP919 order:
bold, italic, underline, obfuscated, strikethrough. Executed MCP919 formatted
text matches &l&n&k&mA&r&l&n&mB&r&l&n&k&mC&r for the regression fixture.
Workspace tests pass. This preserves state and codes only: same-width randomized
animated glyph drawing remains required and is not claimed as implemented.


### Native obfuscated glyph drawing

Tab and scaled title rendering now replace obfuscated printable ASCII glyphs
on each draw using cached native-width candidate groups and a mixed atomic
counter. Original advance and bold spacing are preserved; title mask rebuilding
retains the k code. Tests check equal advances for all 95 printable ASCII inputs
across 128 selector values and changing framebuffer output across repeated Tab
and title draws. The implementation follows MCP919 FontRenderer's same-width
selection principle, but does not yet match its full 256-character candidate
table, Java RNG or Minecraft font metrics. Non-ASCII inputs are currently left
unchanged, and normal chat styled drawing/manual visual parity remain incomplete.
No game glyph assets or reference code are published.


### Java-compatible font random selection

Native obfuscated drawing now uses Java Random's 48-bit state transition, seed
scrambling and nextInt power-of-two/rejection branches. The thread-local runtime
seed varies at startup. Glyph selection draws from printable ASCII and retries
until native width matches, following the FontRenderer selection loop instead
of modulo-selecting a width bucket. Java 8 seed-zero output for four bounds
(95, 2, 1073741825, 2147483647) matches all 32 regression values, including
rejection behavior. Runtime selector width and changing frame tests pass with
the workspace suite. Minecraft's full glyph table/metrics and default Java seed
initialization remain different; exact random animation parity is not claimed.


### Production obfuscation selector regression

Removed the obsolete test-only modulo bucket selector. The same Java-random
rejection selector now serves runtime drawing and deterministic tests. Tests
exercise 128 actual selections for each printable ASCII character (12,160
selections), verify unchanged advances, and verify unsupported non-ASCII input
does not consume RNG state. Candidate widths are cached once, avoiding repeated
font metric computation on rejected candidates while preserving draw order.
The workspace suite and formatting checks pass. This does not broaden the
candidate set or establish Minecraft font/visual parity.


### Color enum input validation

Native display parsing now rejects boolean/array/object color inputs, matching
MCP919 EnumTypeAdapterFactory's JsonReader nextString conversion. Null, numeric
and unknown string inputs remain accepted and inherit parent color; enum names
are case-sensitive, so uppercase RED also inherits rather than selecting red.
A regression failed on boolean color before the fix. An executed MCP919 probe
confirms four rejections and four identical inherited-color code sequences,
matching the native tests. Full workspace tests pass. Exact exception classes
and ingress/disconnect handling still remain incomplete.


### Styled native chat history

Normal chat history now uses formatted component rows and shared native styled
glyph drawing, including inherited colors, bold/italic, underline/strike and
obfuscation. Character wrapping retains each character's resolved style across
spaces/newlines, and history fading blends glyph coverage with its resolved
color. Row spacing/background now accommodate the existing 16-pixel native
styled glyph surface. Regressions cover style retention after wrapping, red/green
framebuffer output, and the actual draw_chat_history path from JSON ChatLine.
Full workspace tests pass. Minecraft pixel-width wrapping, shadows, local font
metrics, click/hover interaction and manual visual parity remain incomplete.
No assets/reference source are published.


### Native styled chat shadows

Chat history now draws a full shadow pass at x+1/y+1 before the foreground
pass, matching GuiNewChat's drawStringWithShadow path and FontRenderer's ordering.
Both passes use the existing source shadow palette (quarter intensity and the
gold exception) and the same age alpha; zero alpha leaves the frame unchanged.
Framebuffer regressions verify additional dark shadow pixels and transparent
behavior, alongside the existing real chat-history style test. The workspace
suite passes. Native font glyphs/scale, pixel-width wrapping and manual visual
parity remain incomplete. No game assets/reference source are published.


### Native glyph-width chat wrapping

Formatted chat history now wraps using actual native glyph advances plus bold
spacing instead of a fixed six-pixel column estimate. Wrapping retains resolved
styles, explicit newlines and word boundaries; a glyph wider than the viewport
occupies a single clipped row so processing still advances. Scroll bounds now
use these same formatted rows. Prefixes are emitted only when style changes.
Regressions cover mixed narrow/wide glyphs, bold increasing row count, per-row
width bounds and runtime row-count agreement. The full workspace suite passes.
The implementation follows GuiUtilRenderComponents' width-driven principle;
Minecraft font metrics, exact component-boundary/space handling, UI scaling and
manual visual parity remain incomplete.


### Production chat row regressions

Removed the obsolete plain fixed-column chat row implementation that survived
as test-only code after styled/pixel wrapping integration. Numeric token, age
and wrapping regressions now read the actual formatted runtime rows; the test
plain-text helper only strips their format codes. Additional production-path
coverage verifies explicit blank lines, retained red/bold style on both sides,
age propagation and exclusion of action-bar messages. Narrow row tests check
rendered advance bounds and text preservation rather than former column counts.
Workspace tests and formatting checks pass. Minecraft font/space-boundary parity
remains incomplete.


### Local ASCII bitmap font in HUD rendering

GameAssets now imports the user's 1.8.9 textures/font/ascii.png into ignored
local_assets; cache completeness includes that file. Native startup installs the
128x128 atlas for printable ASCII HUD glyphs. Glyph advance scans nonzero alpha
columns following FontRenderer.readFontTexture, with the space render advance
handled separately. HUD glyphs use doubled nearest bitmap pixels, and shared
width lookup feeds Tab, title, chat wrapping and obfuscation. Synthetic tests
cover nonzero low alpha, blank glyphs, space and unsupported Unicode. An isolated
real-jar test verifies imported atlas size and every rendered A glyph pixel
against doubled source alpha. It passed; full workspace tests also pass.
This is incomplete font parity: extended atlas mapping, glyph_sizes/Unicode
pages, exact bold/italic geometry, UI scale selection and manual visual parity
remain required. Launcher/input UI still uses the native system font. No font
image or other game assets are published.


### Doubled bitmap bold spacing and geometry

Bitmap ASCII bold now draws the second glyph at two native pixels rather than
one, corresponding to FontRenderer's one-unit default glyph bold offset under
the current doubled atlas scale. Its advance also adds two pixels consistently
in Tab, chat wrapping/composition and title centering; system-font fallback
retains its existing one-pixel spacing. The isolated real-atlas test checks
bold pixels against the composited original glyph and x+2 copy and verifies
ASCII/fallback advance selection. Dedicated and workspace tests pass. Unicode
font paths, exact italic geometry and selectable GUI scale remain incomplete.


### Bitmap shadow scale consistency

All-bitmap ASCII chat/title lines now use a two-pixel native shadow offset,
scaled again by title scale, matching the current doubled glyph rendering.
System-font and mixed fallback lines retain the prior one-pixel offset. The
real-atlas integration test checks the composited chat glyph/shadow pixels
against source alpha at the original and x+2/y+2 positions. Dedicated and full
workspace tests pass. Mixed bitmap/Unicode per-glyph offsets, selectable GUI
scale and full Unicode rendering remain incomplete.


### Local Unicode font pages in HUD drawing

GameAssets imports local glyph_sizes.bin and Unicode page PNGs into ignored
local_assets. The 65,536-byte table is validated; 256x256 pages load lazily with
a shared cache, including missing-page results. Shared HUD width/draw paths now
use BMP glyph nibbles and cropped page alpha rather than the system font, with
zero-width table entries drawing nothing. The width calculation follows
FontRenderer.getCharWidth's wide-glyph normalization. An isolated actual-jar
test verifies Japanese 日 width lookup and every cropped glyph pixel against
the local Unicode page; it passed. Extraction path restrictions and full
workspace tests also pass. Extended default-atlas mapping, UTF-16 supplementary
characters, forced Unicode mode, bold/italic/shadow precision, GUI scaling and
manual visual parity remain incomplete. Missing/malformed page fallback does
not yet reproduce vanilla resource exceptions. No binary table, font image or
other game asset is published.

### Unicode bold offset in normal font mode

MCP919 FontRenderer.renderStringAtPos uses a one-unit bold offset with
unicodeFlag=false and increments the glyph advance by one. At the current
two-pixel HUD scale, local Unicode glyphs now duplicate at x+2 and advance
by two additional pixels. The actual-jar Japanese glyph test checks every
bold composited pixel against the original crop and its x+2 copy; it passes.
Forced Unicode mode and per-glyph shadow placement remain incomplete.

### Unicode measurement versus drawing advance

MCP919 getCharWidth normalizes wide glyphs, while renderStringAtPos advances
by the truncated renderUnicodeChar result without that normalization. The
local official glyph table contains 17,367 differing entries. HUD glyph
placement now uses a separate rendering advance; measurement and wrapping
retain getCharWidth behavior. The actual-jar test checks U+0488 measured
width 18 versus drawing advance 16 at current scale and repeated-glyph
pixel placement. Dedicated and full workspace tests pass. Mixed styled-run
composition and title masks still need separate rendering-advance review.

### Unicode advance across styled runs

Chat and title compositing now position successive styled runs using the
rendering advance, preserving measured widths for wrapping and title
centering. The actual-jar integration test compares every foreground pixel
of two U+0488 glyphs with and without an intervening white formatting code
in chat and scaled title paths. Dedicated and full workspace tests pass.
Per-glyph shadow offsets, forced Unicode mode, default extended atlas mapping
and manual visual parity remain incomplete.

### Per-glyph chat shadow offset in normal mode

MCP919 drawString starts the shadow at +1/+1, but renderStringAtPos
subtracts the normal-mode offset for Unicode glyph rendering. Chat masks
now composite per character: local ASCII glyphs retain +2/+2 at current
scale, local Unicode glyphs use zero net offset. An actual-jar test compares
Japanese shadow coverage and quarter-color pixels with foreground glyph
coverage at the same location. Both Unicode and ASCII dedicated integration
tests and the full workspace suite pass. Underline/strike shadow positioning,
title shadow offsets, extended default atlas mapping, forced Unicode mode
and manual visual parity remain incomplete. Per-character mask composition
has not been performance profiled. No game assets are published.

### Title per-glyph Unicode shadows

Chat and title share glyph shadow offsets. Actual-jar Unicode scaled title
shadow pixel/color comparison and ASCII regression tests pass. Decoration
lines and forced Unicode mode remain incomplete; composition is not profiled.

### Inventory mode-6 collection vertical slice

MCP919 Container.slotClick and GuiContainer were inspected. Player inventory
and unrestricted storage collect partial stacks before full stacks, honor
direction, item/damage/NBT equality and stack limits, skip crafting output
and send a null returned stack. Existing prediction rollback is reused.
GUI matching left clicks within 250 ms suppress the second pickup and submit
collection on release over the same slot through LiveRuntime. E/Escape clears
pending click state. Tests cover priority, direction, occupied-slot no-op and
rejection rollback. The official 1.8.9 server test picked up 10 stone, collected
20+30, placed 60 into slot 13 and received /testfor server-side NBT success.
Dedicated tests and full workspace tests pass. Manual mouse interaction,
Shift double-click, special-container rules and drag remain unverified or
unimplemented. M1 remains in progress.

### Inventory mode-5 drag integration

Container mode-5 transitions and computeStackSize were inspected in MCP919.
InventoryState tracks drag mode and unique selected slots, computes even,
single-item and creative distributions using shared slot rules, caps stacks
and restores cursor/slots on rejected end transactions. Runtime sends the
start/select/end packet sequence. Native GUI defers carried-item placement,
collects eligible slots on pointer movement and submits on matching release;
creative middle drag is connected. E/Escape and container lifecycle clear
drag state. Unit tests cover duplicate slots, remainder, insufficient count,
invalid creative mode/transitions, output/armor restrictions, capacity and
rollback. An isolated official server verified each resulting slot by NBT:
12 => 4/4/4; 12 => 1/1/1 plus 9 remaining; creative => 64/64/64. All passed.
The initial multi-slot NBT command exceeded the protocol chat limit and
failed with tag syntax errors; per-slot commands <=100 characters resolved
this test issue. Manual GUI parity, selected-slot preview, complete special
container/recipe side effects and exact interruption behavior remain open.
No game assets or reference sources are published.

### Shared insertion/merge rules and special-container Shift transfer

Insertion rules now reside in inventory/slot.rs and are shared by pickup and
drag. Known-container collection exclusions feed native double-click gating
and collection prediction; workbench crafting output is excluded, while
furnace/brewing/enchanting/beacon and horse base merge paths are enabled.
Anvil/merchant take conditions remain unsupported rather than assumed true.
Brewing ingredient metadata was checked by executing local MCP919 Item
registration and isPotionIngredient for 432 IDs times 16 metadata values.
All 6,912 cases match, including raw/cooked fish metadata 3 (210 positive
cases total). Five new tests cover ingredient pickup/drag, output exclusion,
special-slot collection, brewing Shift priorities and enchanting split NBT.
Brewing Shift routes to an empty ingredient slot, potion storage or player
storage according to source order. Enchanting Shift handles lapis and the
one-item branch: single-stack NBT is copied; a split larger stack produces
a fresh untagged item. mergeItemStack behavior remains source-driven rather
than globally imposing insertion limits on its separate algorithm.
The isolated official server test confirms brewing 12 and enchanting 1
items after Shift into the special slot, pickup and replacement into player
inventory using server-side /testfor NBT. Earlier drag and collection checks
also pass. The first special-container probe attempted /setblock before the
destination chunk loaded; teleport/chunk readiness before setup fixed the
fixture. Full workspace tests pass. Recipe side effects, enchanting actions,
anvil/merchant costs, complete horse restrictions and manual UI parity are
not complete. No game resources or reference sources are published.

### Local crafting matching, consumption and repeat transfer

MCP919 ShapedRecipes, ShapelessRecipes, SlotCrafting, InventoryCraftResult
and Container retrySlotClick were inspected. The authored Rust crafting
module registers 99 recipes covering basic wood, tools, armor, selected
food/material/dye/compression recipes. Matching handles offsets, horizontal
reflection, wildcard metadata and shapeless multisets, retaining shaped-first
recipe priority. Local input pickup/drag refreshes the result; result pickup
takes the complete output even with right-click, checks cursor capacity,
consumes one item per occupied matrix slot and recomputes the result. Generic
water/lava/milk bucket remainders are returned to the matrix or player storage;
a full inventory leaves authoritative server drop/spawn handling in control.
Shift result pickup repeats while material and inventory capacity allow,
returns the first output stack, consumes each recipe and uses existing
transaction snapshots to restore ingredients, output, storage and cursor
after rejection. Workbench ingredient/storage Shift routes are connected.
Eight normal tests cover matching, duplicate ingredients, whole-output right
pickup, full cursor/storage, repeat craft, bucket returns and rollback. An
executed local Java CraftingManager probe supplies 236 positive/negative
fixtures across log metadata, mixed planks, tools, mirrored armor, wool,
book and cake. The isolated Rust comparison passed all 236. Official 1.8.9
server NBT confirms 2 planks => 4 sticks with right output pickup, and
6 planks => 12 sticks with Shift repeat, after matrix drag placement; both
matrices are exhausted. Earlier inventory scenarios also pass. Full workspace
tests pass. This is a partial recipe catalog, not complete crafting parity:
special recipes, remaining registrations, exact selected-hotbar remainder
priority/drop prediction, throw/swap result handling and statistics are open.
Unsupported recipes require authoritative server result updates; local
prediction does not yet match them. Manual GUI parity remains unverified.
No reference Java sources or game resources are published.

### Repair and book/map cloning

Repair prediction reconstructs RecipesRepairItem: two single damageable items,
matching item type, combined remaining durability plus the integer 5% bonus,
zero-clamped damage and fresh output without enchantment/NBT. The local Java
CraftingManager comparison now passes 2,036 cases, including 1,800 repair cases
across all 50 registered damageable items and six damage values per input.
Book cloning copies compound NBT, updates generation, rejects generation >=2,
counts occupied blank-book slots and retains a single original. Map cloning
retains metadata and display.Name only; other original tags are discarded.
A separate executed MCP919 probe passes 480 book/map cases including NBT,
multiple blank slots/counts, generation limits, duplicate originals and wrong
materials. NBT encoding preserves Java modified UTF-8 including null and
surrogates, with depth/element/byte limits and list-type validation. Normal
unit tests cover NBT encoding and recipe consumption. Stacked original-book
reference sharing/remainder behavior is still open; these cases are not
included in the consumption claim. Remaining special recipes and the full
static catalog are still incomplete.

The isolated official 1.8.9 server confirms repair output damage 1261, two
cloned maps, and a generation-1 cloned book with the original retained.
Each scenario uses actual matrix pickup and output placement, acknowledged
transactions and server-side /testfor Inventory NBT. The first map fixture
used an absent map id which the server normalized; the corrected fixture
uses an existing map id. Both earlier drag/collection/special-slot and basic
crafting scenarios also pass. Full workspace tests and format/diff checks
pass after the final code changes. The owned server stopped and saved.
Manual GUI operation remains unverified.

### Leather armor dye crafting

RecipesArmorDyes, ItemArmor color accessors, EnumDyeColor metadata mapping and
EntitySheep RGB values were inspected locally. Crafting now copies one leather
armor stack, includes an existing int-typed display.color in the average,
combines each occupied dye slot using Java float truncation and brightness
normalization, preserves damage and other NBT, and replaces display.color.
Invalid dye metadata maps to black; non-leather armor and duplicate armor
are rejected. Normal consumption handles the armor and one dye per slot.
An executed MCP919 probe compares 8,550 output/NBT cases: four leather armor
types and an iron armor rejection, five existing-color states, 18 first-dye
metadata values and 19 second-dye states. All match. The official server
confirms a red dyed chestplate with color 10040115, damage 17, exhausted
matrix ingredients and an acknowledged result pickup/storage placement.
Earlier inventory, repair and cloning scenarios also pass in this run.
Full workspace tests and format/diff checks pass. Rendering dyed armor and
manual GUI parity are not yet verified; this is not full M1 completion.

### Fireworks crafting

Local RecipeFireworks and ItemDye were inspected. Runtime crafting now handles
rockets, stars and fade-color updates. Occupied slots determine ingredient
counts; stack counts do not multiply effects. Rockets require one paper and
one to three powder slots, and only receive Flight/Explosions NBT when a star
slot is present. Stars record ordered dye colors, optional Trail/Flicker and
one shape modifier. Fade recipes copy one star and preserve other tags.
An absent or mistyped Explosion on a tagged star follows Java's detached
getCompoundTag behavior: matching succeeds but the output does not gain a
new Explosion tag. Untagged stars cannot be faded. An executed MCP919 probe
compares 9,816 cases, covering dye masking, shape modifiers, powder/paper/star
counts, explosion ordering, invalid ingredients and absent/mistyped tags.
All output item/count/damage and semantic NBT comparisons pass. A normal
unit test exercises star creation, fade update, rocket creation and matrix
consumption. Fireworks entity rendering, effects and usage remain incomplete.

The isolated official 1.8.9 run passes star creation, plain rocket creation
and fade updates via real matrix placement and result pickup. The client
receives acknowledged transactions and resulting Colors/FadeColors arrays,
and server-side Inventory NBT checks confirm output placement and Explosion
Type. Both input slots are exhausted. Earlier inventory/repair/dye/cloning
scenarios also pass. Full workspace tests, all four local Java recipe probes
and format/diff checks pass. The owned server stopped and saved; manual GUI
parity and fireworks use/rendering remain unverified.

### Banner recipes and pattern duplication

RecipesBanners, TileEntityBanner, ItemStack.getSubCompound and NBT list
access/append behavior were inspected. Sixteen plain banner recipes increase
the static catalog to 115. A dedicated inventory/banner.rs implements all
38 craftable patterns in reference order: exact 3x3 dye masks with a shared
metadata value, or a single metadata-matched token and at most one dye slot.
Token recipes without a dye use color zero. Addition copies one banner and
preserves its other NBT, rejects six existing compound patterns, and handles
mistyped Patterns lists without attaching a detached Java getTagList result.
Duplication requires exactly one patterned and one blank banner with matching
base colors, copies one patterned output and returns one patterned original
through the existing remainder/storage path. Existing bucket matching in
that path now compares the actual remainder item/metadata/limit.
The executed MCP919 probe passes 12,832 output/NBT cases: all 38 patterns,
18 dye metadata values, 0/5/6 existing layers, four valid/invalid input variants,
all paired base colors/pattern states/input orders, and 16 plain-banner recipes.
Normal tests cover original return, six-layer rejection, dye-free token
recipes and wrong empty/nonempty list types. Full banner rendering/placement,
manual GUI parity and the remaining M1 requirements remain incomplete.

The isolated official server confirms a dye-free curly-border pattern and
pattern duplication through real matrix clicks, result pickup and inventory
placement. Transactions are acknowledged, the original patterned banner is
retained, the blank/token input is consumed, and /testfor confirms the result
pattern NBT. The initial check exceeded protocol 47's 100-character chat
limit; the corrected isolated-test selector keeps the command within that
limit. All earlier inventory and special recipe scenarios also pass. Full
workspace tests, the five local Java recipe probes (33,714 cases) and
format/diff checks pass. The owned server stopped and saved. No MCP sources
or Minecraft assets are published.

### Complete static recipe registration

MCP919 CraftingManager, ShapedRecipes, ShapelessRecipes and registration
helpers were inspected. A local Java registry audit identifies 373 ordered
registrations: 365 exact-class shaped/shapeless recipes and eight dynamic
recipes. None of the static outputs carry NBT or enable copyIngredientNBT.
The partial hand-maintained 115-entry catalog is replaced by typed Rust facts
for all 365 static registrations, preserving width/height, null cells,
ingredient IDs, exact/wildcard metadata, output quantity/damage and relative
registry order. No runtime Java dependency or game resource is introduced.
The catalog contains recipe facts, not reference Java source. Special recipes
continue to execute authored Rust algorithms. Static/dynamic matching uses
original global registry positions, with map extension at position 72 still
unimplemented.
An executed probe visits every static registration and supplies 14,336 cases
across 2x2/3x3 grids, fitting offsets, horizontal mirrors, shapeless shifts and
reversal, wildcard substitutions and changed ingredients. Expected results
come from the whole Java CraftingManager, including competing recipes. All
Rust results match. The six executed recipe comparisons total 48,050 cases
and all pass. This proves the tested registration/matching scenarios, not
all crafting side effects or M1 completion. World-dependent map extension,
result throw/swap, statistics/achievements, exact remainder edge cases and
manual GUI parity are still open.

The official server run also confirms newly registered granite (one item,
damage 1) and andesite (two items, damage 5) through matrix placement, result
pickup and storage. Ingredient slots are exhausted and server-side NBT
queries confirm quantity/damage. Earlier special-recipe scenarios also pass.
After preserving combined recipe registry positions, all six Java probes
and full workspace tests pass again. The owned server stopped and saved.
Format/diff and publication checks pass; no assets or reference sources are
included. This remains partial progress toward full Minecraft compatibility.

### Crafting result throws

Container mode 4, SlotCrafting and InventoryCraftResult.decrStackSize were
inspected. Player/workbench result throws now take the entire result stack
for either button, consume one recipe, apply remainders and recompute the
result. Throwing an input slot also recomputes the result. A carried stack
continues to block mode-4 mutation. Prediction stores the full pre-click
snapshot for transaction rejection; workbench/player aliases synchronize.
Normal tests cover both buttons, repeat-material result refresh, input throw,
carried-stack suppression and player/workbench rollback. Anvil/merchant/
furnace result side effects and result number-key swaps remain incomplete.

The official server confirms both result-throw buttons drop an Item entity
containing four sticks and exhaust both plank matrix slots, with no carried
stack. The full earlier inventory/special-recipe run passes as well. The
expanded command count exposed a test-only bounded-chat-history cursor bug:
length-based skip stopped seeing responses after 100 lines. Per-query chat
markers now delimit new confirmations across history rotation; the corrected
full run passes. Full workspace tests and format/diff checks pass. The owned
server stopped and saved. Manual GUI parity and other result-slot side
effects remain unverified; this is not full inventory compatibility.

### Crafting result number-key pickup

Container mode 2 was inspected. Player/workbench results now move their full
stack into the selected hotbar slot and execute crafting pickup side effects.
An occupied hotbar stack is rehomed through existing inventory insertion;
a full inventory with no empty slot prevents the transfer, matching the
reference precondition. Matrix/result/storage snapshots restore rejected
transactions. Player crafting input swaps now retain the changed matrix,
rather than discarding the independently updated container snapshot, and
refresh the output. Workbench/player hotbar aliases synchronize before and
after recipe consumption so remainder storage is retained. Normal tests
cover empty/occupied hotbar pickup, full-storage suppression, input result
refresh, player/workbench rejection restoration and mirrored hotbar slots.
Anvil/merchant result swaps and other pending M1 requirements remain open.

The isolated official server confirms four crafted sticks in hotbar slot 0
with both empty and occupied destinations. Both matrix inputs are consumed;
in the occupied case the original twelve stone move to hotbar slot 1.
Transactions are acknowledged and server Inventory NBT checks confirm the
result quantity and slot. All earlier inventory and special-recipe scenarios
also pass. Full workspace tests and format/diff checks pass. The owned server
stopped and saved. Manual GUI parity and other result-slot side effects are
still unverified; full compatibility remains incomplete.
