# MCP919 source-led reconstruction

The objective is to understand the local MCP919 client and reconstruct its behavior in Rust. Folder correspondence is a navigation and ownership constraint, not evidence of completed compatibility. Minecraft source and assets remain ignored local inputs.

## Responsibility map

| MCP919 package / class | Current Rust implementation | Structural work still required |
| --- | --- | --- |
| client/Minecraft | rmc-client main, shell, play_cli, live_runtime | Separate application loop, screen lifecycle and input orchestration |
| client/network/NetHandlerPlayClient | rmc-client/client/network/net_handler_play_client.rs; session, live_runtime | Shared world, teleport, explosion and combat dispatch extracted; join/respawn, entities, inventory and abilities still distributed |
| client/multiplayer/PlayerControllerMP | rmc-game combat, mining, inventory; live_runtime actions | One controller owner for held-item synchronization and ordered interaction decisions |
| client/entity/EntityPlayerSP | rmc-game simulation, player; rmc-client shell | Separate local player update and walking packet emission from inherited physics |
| entity/EntityLivingBase; entity/Entity | rmc-game simulation; rmc-world collision/environment | Represent living travel and base collision/motion as separate responsibilities |
| client/multiplayer/WorldClient; world/World; world/chunk/Chunk | rmc-world WorldSnapshot; client/entity/player.rs EntityTracker | Separate world lifecycle, chunk storage and entity ownership |
| inventory/Container; inventory/ContainerPlayer; entity/player/InventoryPlayer | rmc-game inventory/mod.rs, inventory/container.rs, inventory/slot.rs and inventory/crafting.rs | Pickup, number-key swap, creative clone and throw algorithms separated from state; slot rules, player storage and recipe side effects still need reconstruction |
| network/NetworkManager; network/PacketBuffer | rmc-net transport, driver, buffer, framing | Preserve transport/codec boundary; distinguish delivery from gameplay effects |
| network/play/client; network/play/server | rmc-net codec/play and protocol/play_* | Split implemented packet definitions along the source package hierarchy |
| client/renderer/EntityRenderer; RenderGlobal | rmc-render; play_cli | Separate picking, world render orchestration, terrain and entity submission |
| client/renderer/BlockRendererDispatcher; BlockModelShapes; client/resources | rmc-render meshes; play_assets | Reconstruct blockstate/model/resource loading; current cube meshes do not cover these classes |
| client/gui; client/audio | rmc-ui; play_cli; usability | Separate screens, HUD and sound ownership; full behavior remains incomplete |

The shared network handler and inventory container algorithms have been physically separated. Other rows record existing code and required restructuring, not implemented modules.

## Call chains inspected locally

* Minecraft.runGameLoop updates Timer, drains scheduled tasks, executes elapsed runTick calls, then prepares sound and rendering. Fixed simulation ticks and rendered frames are distinct.
* Minecraft.runTick performs GUI work and picking, then PlayerControllerMP.updateController before later input and world/player work. The current Rust aggregate loop must be audited against this order rather than assumed equivalent.
* EntityPlayerSP.onUpdate invokes the inherited update before onUpdateWalkingPlayer. Its onLivingUpdate runs local input, sprint and flight decisions before the inherited living update. Rust composition must preserve that distinction.
* PlayerControllerMP.attackEntity synchronizes the selected item before sending the attack. Its windowClick delegates local prediction to Container.slotClick and submits the returned stack with the transaction number. Container and controller are different owners.
* NetHandlerPlayClient uses PacketThreadUtil.checkThreadAndEnqueue to transfer effects to the main thread. handlePlayerPosLook resolves relative components, clears motion on absolute axes, updates the player, acknowledges and finishes initial terrain loading. Current Rust acknowledgement and simulation effects are split across session and runtime; extraction alone does not prove timing parity.
* NetHandlerPlayClient.handleExplosion applies world explosion effects before adding player motion. The shared Rust handler applies world changes before appending additive velocity and combat effects, preserving its existing order. Particles and audio are not reconstructed by this extraction.

## Implementation rule

For each next responsibility: read its source callers and state owners, document the transition and order, move the real implementation into the corresponding package, then compare behavior. Keep compatibility imports only when they protect existing consumers. Do not substitute empty class-shaped files for implementation, or copy reference code into publication paths.

The first extraction removes identical inbound-effect functions from live_cli and live_runtime. Both now invoke client/network/net_handler_play_client, so fixes to these effects share one implementation. This is a starting boundary, not a complete port of NetHandlerPlayClient.

## Map data and crafting context

S34PacketMaps maps to rmc-net/codec/play/maps.rs and rmc-world/maps.rs.
NetHandlerPlayClient.handleMaps updates scale, icons and a bounded pixel
rectangle. RecipesMapExtending requires ItemMap.getMapData and scale < 4;
InventoryState keeps received scales across open windows and rejected clicks.
ItemMap.onCreated / SlotCrafting.onCrafting immediate local map allocation
and statistics remain open; server-assigned IDs reconcile after closing.

## Statistics and achievement synchronization

S37PacketStatistics and StatList.getOneShotStat map to rmc-net/codec/play/
statistics.rs and statistics_registry.rs (identifier/flag facts only).
NetHandlerPlayClient.handleStatistics and StatFileWriter absolute values
map to rmc-game/statistics.rs through UsabilityState. LiveRuntime exposes
ClientStatus(1) requests. First-response achievement suppression and the
exact previous-value == 0 notification condition follow the handler.
Native toast rendering, statistics screens, option persistence, independent
local increments and parent-achievement logic remain open.

## Achievement dependencies and local increments

AchievementList registry facts map to rmc-game/achievement_catalog.rs.
StatFileWriter.hasAchievementUnlocked/canUnlockAchievement/func_150874_c
map to StatisticsState achievement queries. StatFileWriter.increaseStat
maps to StatisticsState.increase with wrapping Java int addition and parent
gating. EntityPlayerSP.addStat additionally accepts only independent IDs;
18 such IDs are registered in statistics_registry.rs. Gameplay action
callers, rendering and persistence remain open.

EntityPlayer.jump -> triggerAchievement(StatList.jumpStat) now maps to
LocalSimulationLayer.apply_jump -> ClientShell.statistic_increments ->
LiveRuntime StatisticsState.increase with the remote-player gate. Water/flight
vertical input follows separate simulation branches and does not emit jump.
