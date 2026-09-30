# Reference Index

This file exists to keep future implementation work context-efficient. Start with this reference set before opening anything else.

Non-Java companion spec:

- `docs/protocol-contract.md`

Packet contract foundation:

- `MCP-919/src/minecraft/net/minecraft/network/EnumConnectionState.java`
- `MCP-919/src/minecraft/net/minecraft/network/PacketBuffer.java`

## MCP 1.8.9

### Loop and timing

- `MCP-919/src/minecraft/net/minecraft/client/Minecraft.java`
  - `runGameLoop` around line 1083
  - `runTick` around line 1741
- `MCP-919/src/minecraft/net/minecraft/util/Timer.java`
  - `updateTimer` around line 65

Rust owner:

- `rmc-client`

### Local player movement

- `MCP-919/src/minecraft/net/minecraft/client/entity/EntityPlayerSP.java`
  - `onUpdateWalkingPlayer` around line 189
  - `onLivingUpdate` around line 715
- `MCP-919/src/minecraft/net/minecraft/util/MovementInputFromOptions.java`
  - movement aggregation around lines 16-45

Rust owner:

- `rmc-game`

### Interaction, combat, and inventory

- `MCP-919/src/minecraft/net/minecraft/client/multiplayer/PlayerControllerMP.java`
  - `clickBlock` around line 198
  - `syncCurrentPlayItem` around line 379
  - `onPlayerRightClick` around line 385
  - `sendUseItem` around line 456
  - `attackEntity` around line 495
  - `interactWithEntitySendPacket` around line 509
  - `isPlayerRightClickingOnEntity` around line 521
  - `windowClick` around line 534
  - `onStoppedUsingItem` around line 576
- `MCP-919/src/minecraft/net/minecraft/client/entity/EntityPlayerSP.java`
  - `swingItem` around line 304
  - `setPlayerSPHealth` around line 328

Rust owner:

- `rmc-game`

### Network apply path

- `MCP-919/src/minecraft/net/minecraft/client/network/NetHandlerPlayClient.java`
  - `handleJoinGame` around line 277
  - `handleEntityVelocity` around line 501
  - `handlePlayerPosLook` around line 669
  - `handleUpdateHealth` around line 1042
  - `handleRespawn` around line 1056
  - `handleWindowItems` around line 1189
  - `handleConfirmTransaction` around line 1174
  - `handleKeepAlive` around line 1663

Rust owners:

- `rmc-net`
- `rmc-world`
- `rmc-game`

### World state

- `MCP-919/src/minecraft/net/minecraft/client/multiplayer/WorldClient.java`
  - `tick` around line 66
  - `doPreChunk` around line 153
  - `addEntityToWorld` around line 234
  - `removeEntityFromWorld` around line 262
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S21PacketChunkData.java`
  - `getExtractedData` around line 78
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S22PacketMultiBlockChange.java`
  - `readPacketData` around line 32
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S23PacketBlockChange.java`
  - `readPacketData` around line 26
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S26PacketMapChunkBulk.java`
  - `readPacketData` around line 39
- `MCP-919/src/minecraft/net/minecraft/world/chunk/Chunk.java`
  - `fillChunk` around line 1307
- `MCP-919/src/minecraft/net/minecraft/world/chunk/storage/ExtendedBlockStorage.java`
  - `get` around line 45
  - `set` around line 51

Rust owner:

- `rmc-world`

### Packet classes for M4

- `MCP-919/src/minecraft/net/minecraft/network/play/client/C02PacketUseEntity.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/client/C07PacketPlayerDigging.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/client/C08PacketPlayerBlockPlacement.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/client/C0BPacketEntityAction.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/client/C0EPacketClickWindow.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/client/C0FPacketConfirmTransaction.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S06PacketUpdateHealth.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S07PacketRespawn.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S12PacketEntityVelocity.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S30PacketWindowItems.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S32PacketConfirmTransaction.java`

Rust owners:

- `rmc-net`
- `rmc-game`

### Rendering

- `MCP-919/src/minecraft/net/minecraft/client/renderer/EntityRenderer.java`
  - `getMouseOver` around line 409
  - `renderWorld` around line 1280
  - `renderWorldPass` around line 1323
- `MCP-919/src/minecraft/net/minecraft/client/Minecraft.java`
  - `runGameLoop`
  - `runTick`
- `MCP-919/src/minecraft/net/minecraft/client/renderer/RenderGlobal.java`
  - `loadRenderers` around line 482
  - `renderEntities` around line 556
  - `setupTerrain` around line 774
  - `markBlockRangeForRenderUpdate` around line 1992
- `MCP-919/src/minecraft/net/minecraft/client/renderer/chunk/ChunkRenderDispatcher.java`
  - chunk rebuild scheduling and upload queue reference
- `MCP-919/src/minecraft/net/minecraft/util/Timer.java`
  - frame pacing and partial tick behavior

Rust owner:

- `rmc-render`
- `rmc-game::tick`
- `rmc-net::trace`

### GUI

- `MCP-919/src/minecraft/net/minecraft/client/gui/GuiScreen.java`
  - `handleMouseInput` around line 604
  - `handleKeyboardInput` around line 641
- `MCP-919/src/minecraft/net/minecraft/client/network/NetHandlerPlayClient.java`
  - `handleChat`
  - `handleOpenWindow`
  - `handleSetSlot`
  - `handlePlayerListItem`
  - `handleScoreboardObjective`
  - `handleUpdateScore`
  - `handleDisplayScoreboard`
  - `handleTeams`
  - `handleSoundEffect`
- `MCP-919/src/minecraft/net/minecraft/client/gui/inventory/GuiInventory.java`
- `MCP-919/src/minecraft/net/minecraft/client/gui/inventory/GuiChest.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S02PacketChat.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S29PacketSoundEffect.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S2DPacketOpenWindow.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S2EPacketCloseWindow.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S2FPacketSetSlot.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S38PacketPlayerListItem.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S3BPacketScoreboardObjective.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S3CPacketUpdateScore.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S3DPacketDisplayScoreboard.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/server/S3EPacketTeams.java`
- `MCP-919/src/minecraft/net/minecraft/network/play/client/C0DPacketCloseWindow.java`

Rust owner:

- `rmc-ui`
- `rmc-game::usability`
- `rmc-game::inventory`

## MCProtocolLib

Use only as a constrained reference, not as the direct runtime base.

- `MCProtocolLib-master/MCProtocolLib-master/protocol/src/main/java/org/geysermc/mcprotocollib/protocol/MinecraftProtocol.java`
  - codec and state-architecture reference
- `MCProtocolLib-master/MCProtocolLib-master/protocol/src/main/java/org/geysermc/mcprotocollib/protocol/data/ProtocolState.java`
  - shows a modern state model with `CONFIGURATION`

Allowed uses:

- Packet codec design reference
- Tool or generator reference
- Test harness reference

Not a direct use:

- Runtime foundation for a 1.8.9 Hypixel client

## Investigation order

1. `Minecraft.java`
2. `Timer.java`
3. `EntityPlayerSP.java`
4. `PlayerControllerMP.java`
5. `NetHandlerPlayClient.java`
6. `WorldClient.java`
7. `EntityRenderer.java`
8. `RenderGlobal.java`
9. GUI files
