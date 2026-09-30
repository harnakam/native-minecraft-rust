use std::fs;
use std::path::{Path, PathBuf};

const TRACE_HELPER_TEMPLATE: &str = include_str!("../templates/RmcTraceLogger.java");
const TRACE_SCENARIO_TEMPLATE: &str = include_str!("../templates/RmcVerificationScenarios.java");
const LIVE_VERIFICATION_TEMPLATE: &str = include_str!("../templates/RmcLiveVerification.java");
const VERIFICATION_START_TEMPLATE: &str = include_str!("../templates/RmcVerificationStart.java");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaTraceInstallOptions {
    pub mcp_root: PathBuf,
}

impl JavaTraceInstallOptions {
    pub fn default_workspace() -> Self {
        Self {
            mcp_root: PathBuf::from("MCP-919"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JavaTraceInstallSummary {
    pub helper_path: PathBuf,
    pub scenario_path: PathBuf,
    pub backup_root: PathBuf,
    pub modified_files: Vec<PathBuf>,
}

pub fn install_mcp_java_trace(
    options: &JavaTraceInstallOptions,
) -> Result<JavaTraceInstallSummary, String> {
    let src_root = options.mcp_root.join("src").join("minecraft");

    if !src_root.exists() {
        return Err(format!(
            "MCP source root was not found: {}",
            src_root.display()
        ));
    }

    let backup_root = options.mcp_root.join(".rmc-trace-backup");
    fs::create_dir_all(&backup_root)
        .map_err(|error| format!("failed to create {}: {error}", backup_root.display()))?;

    let helper_path = src_root
        .join("net")
        .join("minecraft")
        .join("client")
        .join("rmc")
        .join("RmcTraceLogger.java");
    let scenario_path = src_root
        .join("net")
        .join("minecraft")
        .join("client")
        .join("rmc")
        .join("RmcVerificationScenarios.java");
    let live_verification_path = src_root
        .join("net")
        .join("minecraft")
        .join("client")
        .join("rmc")
        .join("RmcLiveVerification.java");
    let verification_start_path = src_root
        .join("net")
        .join("minecraft")
        .join("client")
        .join("rmc")
        .join("RmcVerificationStart.java");
    ensure_parent_dir(&helper_path)?;
    fs::write(&helper_path, TRACE_HELPER_TEMPLATE).map_err(|error| {
        format!(
            "failed to write Java trace helper {}: {error}",
            helper_path.display()
        )
    })?;
    fs::write(&scenario_path, TRACE_SCENARIO_TEMPLATE).map_err(|error| {
        format!(
            "failed to write Java trace scenario {}: {error}",
            scenario_path.display()
        )
    })?;
    fs::write(&live_verification_path, LIVE_VERIFICATION_TEMPLATE).map_err(|error| {
        format!(
            "failed to write Java live verification helper {}: {error}",
            live_verification_path.display()
        )
    })?;
    fs::write(&verification_start_path, VERIFICATION_START_TEMPLATE).map_err(|error| {
        format!(
            "failed to write Java verification launcher {}: {error}",
            verification_start_path.display()
        )
    })?;

    let mut modified_files = Vec::new();
    for relative_path in patch_targets() {
        let full_path = src_root.join(relative_path);
        let original = fs::read_to_string(&full_path)
            .map_err(|error| format!("failed to read {}: {error}", full_path.display()))?;
        let updated = patch_file(relative_path, &original)?;

        if updated != original {
            let backup_path = backup_root.join(relative_path);
            ensure_parent_dir(&backup_path)?;
            if !backup_path.exists() {
                fs::write(&backup_path, original.as_bytes()).map_err(|error| {
                    format!("failed to write backup {}: {error}", backup_path.display())
                })?;
            }
            fs::write(&full_path, updated.as_bytes())
                .map_err(|error| format!("failed to write {}: {error}", full_path.display()))?;
            modified_files.push(full_path);
        }
    }

    Ok(JavaTraceInstallSummary {
        helper_path,
        scenario_path,
        backup_root,
        modified_files,
    })
}

fn patch_targets() -> &'static [&'static str] {
    &[
        "Start.java",
        "net/minecraft/util/MessageSerializer.java",
        "net/minecraft/util/MessageDeserializer.java",
        "net/minecraft/network/NettyCompressionEncoder.java",
        "net/minecraft/network/NettyCompressionDecoder.java",
        "net/minecraft/client/entity/EntityPlayerSP.java",
        "net/minecraft/client/Minecraft.java",
        "net/minecraft/client/multiplayer/PlayerControllerMP.java",
        "net/minecraft/client/network/NetHandlerPlayClient.java",
    ]
}

fn patch_file(relative_path: &str, source: &str) -> Result<String, String> {
    let mut normalized = source.replace("\r\n", "\n");
    normalized = match relative_path {
        "Start.java" => patch_start(&normalized)?,
        "net/minecraft/util/MessageSerializer.java" => patch_message_serializer(&normalized)?,
        "net/minecraft/util/MessageDeserializer.java" => patch_message_deserializer(&normalized)?,
        "net/minecraft/network/NettyCompressionEncoder.java" => {
            patch_compression_encoder(&normalized)?
        }
        "net/minecraft/network/NettyCompressionDecoder.java" => {
            patch_compression_decoder(&normalized)?
        }
        "net/minecraft/client/entity/EntityPlayerSP.java" => patch_entity_player_sp(&normalized)?,
        "net/minecraft/client/Minecraft.java" => patch_minecraft(&normalized)?,
        "net/minecraft/client/multiplayer/PlayerControllerMP.java" => {
            patch_player_controller_mp(&normalized)?
        }
        "net/minecraft/client/network/NetHandlerPlayClient.java" => {
            patch_net_handler_play_client(&normalized)?
        }
        other => return Err(format!("unsupported MCP patch target: {other}")),
    };

    if source.contains("\r\n") {
        Ok(normalized.replace('\n', "\r\n"))
    } else {
        Ok(normalized)
    }
}

fn patch_start(source: &str) -> Result<String, String> {
    replace_once(
        source,
        "        Main.main(concat(new String[] {\"--version\", \"mcp\", \"--accessToken\", \"0\", \"--assetsDir\", \"assets\", \"--assetIndex\", \"1.8\", \"--userProperties\", \"{}\"}, args));\n",
        "        String verifyHost = System.getProperty(\"rmc.verify.host\", \"\");\n        String verifyPort = System.getProperty(\"rmc.verify.port\", \"25565\");\n        String verifyUsername = System.getProperty(\"rmc.verify.username\", \"\");\n        String verifyWidth = System.getProperty(\"rmc.verify.width\", \"\");\n        String verifyHeight = System.getProperty(\"rmc.verify.height\", \"\");\n        String accessToken = System.getProperty(\"rmc.verify.access_token\", \"0\");\n        String[] launchArgs = concat(new String[] {\"--version\", \"mcp\", \"--accessToken\", accessToken, \"--assetsDir\", \"assets\", \"--assetIndex\", \"1.8\", \"--userProperties\", \"{}\"}, args);\n\n        if (!verifyUsername.isEmpty())\n        {\n            launchArgs = concat(new String[] {\"--username\", verifyUsername}, launchArgs);\n        }\n\n        if (!verifyWidth.isEmpty() && !verifyHeight.isEmpty())\n        {\n            launchArgs = concat(new String[] {\"--width\", verifyWidth, \"--height\", verifyHeight}, launchArgs);\n        }\n\n        if (!verifyHost.isEmpty())\n        {\n            launchArgs = concat(new String[] {\"--server\", verifyHost, \"--port\", verifyPort}, launchArgs);\n        }\n\n        Main.main(launchArgs);\n",
    )
}

fn patch_message_serializer(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import net.minecraft.network.play.server.S0CPacketSpawnPlayer;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = replace_once(
        &updated,
        "            PacketBuffer packetbuffer = new PacketBuffer(p_encode_3_);\n            packetbuffer.writeVarIntToBuffer(integer.intValue());\n",
        "            PacketBuffer packetbuffer = new PacketBuffer(p_encode_3_);\n            int rmcTraceStart = p_encode_3_.writerIndex();\n            packetbuffer.writeVarIntToBuffer(integer.intValue());\n",
    )?;
    replace_once(
        &updated,
        "            catch (Throwable throwable)\n            {\n                logger.error((Object)throwable);\n            }\n",
        "            catch (Throwable throwable)\n            {\n                logger.error((Object)throwable);\n            }\n\n            if (p_encode_1_.channel().pipeline().get(\"compress\") == null)\n            {\n                RmcTraceLogger.tracePacket(\"Serverbound\", (EnumConnectionState)p_encode_1_.channel().attr(NetworkManager.attrKeyConnectionState).get(), integer.intValue(), p_encode_2_.getClass().getSimpleName(), p_encode_3_.writerIndex() - rmcTraceStart, \"Disabled\");\n            }\n",
    )
}

fn patch_message_deserializer(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import net.minecraft.network.PacketBuffer;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = replace_once(
        &updated,
        "        if (p_decode_2_.readableBytes() != 0)\n        {\n            PacketBuffer packetbuffer = new PacketBuffer(p_decode_2_);\n",
        "        if (p_decode_2_.readableBytes() != 0)\n        {\n            int rmcTraceLength = p_decode_2_.readableBytes();\n            PacketBuffer packetbuffer = new PacketBuffer(p_decode_2_);\n",
    )?;
    replace_once(
        &updated,
        "                else\n                {\n                    p_decode_3_.add(packet);\n\n                    if (logger.isDebugEnabled())\n                    {\n                        logger.debug(RECEIVED_PACKET_MARKER, \" IN: [{}:{}] {}\", new Object[] {p_decode_1_.channel().attr(NetworkManager.attrKeyConnectionState).get(), Integer.valueOf(i), packet.getClass().getName()});\n                    }\n                }\n",
        "                else\n                {\n                    if (p_decode_1_.channel().pipeline().get(\"decompress\") == null)\n                    {\n                        RmcTraceLogger.tracePacket(\"Clientbound\", (EnumConnectionState)p_decode_1_.channel().attr(NetworkManager.attrKeyConnectionState).get(), i, packet.getClass().getSimpleName(), rmcTraceLength, \"Disabled\");\n                    }\n\n                    p_decode_3_.add(packet);\n\n                    if (logger.isDebugEnabled())\n                    {\n                        logger.debug(RECEIVED_PACKET_MARKER, \" IN: [{}:{}] {}\", new Object[] {p_decode_1_.channel().attr(NetworkManager.attrKeyConnectionState).get(), Integer.valueOf(i), packet.getClass().getName()});\n                    }\n                }\n",
    )
}

fn patch_compression_encoder(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import java.util.zip.Deflater;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = replace_once(
        &updated,
        "        int i = p_encode_2_.readableBytes();\n        PacketBuffer packetbuffer = new PacketBuffer(p_encode_3_);\n",
        "        int i = p_encode_2_.readableBytes();\n        PacketBuffer rmcTraceBuffer = new PacketBuffer(p_encode_2_.duplicate());\n        int rmcTracePacketId = rmcTraceBuffer.readVarIntFromBuffer();\n        RmcTraceLogger.tracePacket(\"Serverbound\", (EnumConnectionState)p_encode_1_.channel().attr(NetworkManager.attrKeyConnectionState).get(), rmcTracePacketId, \"Packet\" + rmcTracePacketId, i, i < this.treshold ? \"Uncompressed\" : \"Compressed\");\n        PacketBuffer packetbuffer = new PacketBuffer(p_encode_3_);\n",
    )?;
    Ok(updated)
}

fn patch_compression_decoder(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import java.util.zip.Inflater;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = replace_once(
        &updated,
        "            if (i == 0)\n            {\n                p_decode_3_.add(packetbuffer.readBytes(packetbuffer.readableBytes()));\n            }\n",
        "            if (i == 0)\n            {\n                ByteBuf rmcTracePacket = packetbuffer.readBytes(packetbuffer.readableBytes());\n                PacketBuffer rmcTraceBuffer = new PacketBuffer(rmcTracePacket.duplicate());\n                int rmcTracePacketId = rmcTraceBuffer.readVarIntFromBuffer();\n                RmcTraceLogger.tracePacket(\"Clientbound\", (EnumConnectionState)p_decode_1_.channel().attr(net.minecraft.network.NetworkManager.attrKeyConnectionState).get(), rmcTracePacketId, \"Packet\" + rmcTracePacketId, rmcTracePacket.readableBytes(), \"Uncompressed\");\n                p_decode_3_.add(rmcTracePacket);\n            }\n",
    )?;
    replace_once(
        &updated,
        "                byte[] abyte = new byte[packetbuffer.readableBytes()];\n                packetbuffer.readBytes(abyte);\n                this.inflater.setInput(abyte);\n                byte[] abyte1 = new byte[i];\n                this.inflater.inflate(abyte1);\n                p_decode_3_.add(Unpooled.wrappedBuffer(abyte1));\n                this.inflater.reset();\n",
        "                byte[] abyte = new byte[packetbuffer.readableBytes()];\n                packetbuffer.readBytes(abyte);\n                this.inflater.setInput(abyte);\n                byte[] abyte1 = new byte[i];\n                this.inflater.inflate(abyte1);\n                ByteBuf rmcTracePacket = Unpooled.wrappedBuffer(abyte1);\n                PacketBuffer rmcTraceBuffer = new PacketBuffer(rmcTracePacket.duplicate());\n                int rmcTracePacketId = rmcTraceBuffer.readVarIntFromBuffer();\n                RmcTraceLogger.tracePacket(\"Clientbound\", (EnumConnectionState)p_decode_1_.channel().attr(net.minecraft.network.NetworkManager.attrKeyConnectionState).get(), rmcTracePacketId, \"Packet\" + rmcTracePacketId, rmcTracePacket.readableBytes(), \"Compressed\");\n                p_decode_3_.add(rmcTracePacket);\n                this.inflater.reset();\n",
    )
}

fn patch_entity_player_sp(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import net.minecraft.client.network.NetHandlerPlayClient;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = replace_once(
        &updated,
        "            if (this.ridingEntity == null)\n            {\n",
        "            String rmcTracePacket = \"Player\";\n\n            if (this.ridingEntity == null)\n            {\n",
    )?;
    updated = replace_once(
        &updated,
        "                    this.sendQueue.addToSendQueue(new C03PacketPlayer.C06PacketPlayerPosLook(this.posX, this.getEntityBoundingBox().minY, this.posZ, this.rotationYaw, this.rotationPitch, this.onGround));\n",
        "                    this.sendQueue.addToSendQueue(new C03PacketPlayer.C06PacketPlayerPosLook(this.posX, this.getEntityBoundingBox().minY, this.posZ, this.rotationYaw, this.rotationPitch, this.onGround));\n                    rmcTracePacket = \"PlayerPositionAndLook\";\n",
    )?;
    updated = replace_once(
        &updated,
        "                    this.sendQueue.addToSendQueue(new C03PacketPlayer.C04PacketPlayerPosition(this.posX, this.getEntityBoundingBox().minY, this.posZ, this.onGround));\n",
        "                    this.sendQueue.addToSendQueue(new C03PacketPlayer.C04PacketPlayerPosition(this.posX, this.getEntityBoundingBox().minY, this.posZ, this.onGround));\n                    rmcTracePacket = \"PlayerPosition\";\n",
    )?;
    updated = replace_once(
        &updated,
        "                    this.sendQueue.addToSendQueue(new C03PacketPlayer.C05PacketPlayerLook(this.rotationYaw, this.rotationPitch, this.onGround));\n",
        "                    this.sendQueue.addToSendQueue(new C03PacketPlayer.C05PacketPlayerLook(this.rotationYaw, this.rotationPitch, this.onGround));\n                    rmcTracePacket = \"PlayerLook\";\n",
    )?;
    updated = replace_once(
        &updated,
        "                this.sendQueue.addToSendQueue(new C03PacketPlayer.C06PacketPlayerPosLook(this.motionX, -999.0D, this.motionZ, this.rotationYaw, this.rotationPitch, this.onGround));\n                flag2 = false;\n",
        "                this.sendQueue.addToSendQueue(new C03PacketPlayer.C06PacketPlayerPosLook(this.motionX, -999.0D, this.motionZ, this.rotationYaw, this.rotationPitch, this.onGround));\n                rmcTracePacket = \"PlayerPositionAndLook\";\n                flag2 = false;\n",
    )?;
    replace_once(
        &updated,
        "            if (flag3)\n            {\n                this.lastReportedYaw = this.rotationYaw;\n                this.lastReportedPitch = this.rotationPitch;\n            }\n",
        "            if (flag3)\n            {\n                this.lastReportedYaw = this.rotationYaw;\n                this.lastReportedPitch = this.rotationPitch;\n            }\n\n            RmcTraceLogger.traceMovementState(this.ticksExisted, this.posX, this.getEntityBoundingBox().minY, this.posZ, this.motionX, this.motionY, this.motionZ, this.rotationYaw, this.rotationPitch, this.onGround, this.isSprinting(), this.isSneaking(), rmcTracePacket, flag2, flag3);\n",
    )
}

fn patch_minecraft(source: &str) -> Result<String, String> {
    let updated = ensure_import(
        source,
        "import net.minecraft.client.multiplayer.WorldClient;\n",
        "import net.minecraft.client.rmc.RmcLiveVerification;\n",
    )?;
    replace_once(
        &updated,
        "        this.mcProfiler.endSection();\n        this.systemTime = getSystemTime();\n",
        "        RmcLiveVerification.tick(this);\n        this.mcProfiler.endSection();\n        this.systemTime = getSystemTime();\n",
    )
}

fn patch_player_controller_mp(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import net.minecraft.client.network.NetHandlerPlayClient;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = replace_once(
        &updated,
        "            this.currentPlayerItem = i;\n            this.netClientHandler.addToSendQueue(new C09PacketHeldItemChange(this.currentPlayerItem));\n",
        "            this.currentPlayerItem = i;\n            this.netClientHandler.addToSendQueue(new C09PacketHeldItemChange(this.currentPlayerItem));\n            RmcTraceLogger.traceInventoryHeldItem(this.currentPlayerItem);\n",
    )?;
    updated = replace_once(
        &updated,
        "        this.syncCurrentPlayItem();\n        this.netClientHandler.addToSendQueue(new C02PacketUseEntity(targetEntity, C02PacketUseEntity.Action.ATTACK));\n",
        "        this.syncCurrentPlayItem();\n        this.netClientHandler.addToSendQueue(new C02PacketUseEntity(targetEntity, C02PacketUseEntity.Action.ATTACK));\n        RmcTraceLogger.traceCombatAction(\"attack\", targetEntity.getEntityId(), this.currentPlayerItem, playerIn.isSprinting());\n",
    )?;
    updated = replace_once(
        &updated,
        "            this.syncCurrentPlayItem();\n            this.netClientHandler.addToSendQueue(new C08PacketPlayerBlockPlacement(playerIn.inventory.getCurrentItem()));\n",
        "            this.syncCurrentPlayItem();\n            this.netClientHandler.addToSendQueue(new C08PacketPlayerBlockPlacement(playerIn.inventory.getCurrentItem()));\n            RmcTraceLogger.traceCombatUseItem(playerIn.inventory.currentItem, itemStackIn);\n",
    )?;
    updated = replace_once(
        &updated,
        "        short short1 = playerIn.openContainer.getNextTransactionID(playerIn.inventory);\n        ItemStack itemstack = playerIn.openContainer.slotClick(slotId, mouseButtonClicked, mode, playerIn);\n        this.netClientHandler.addToSendQueue(new C0EPacketClickWindow(windowId, slotId, mouseButtonClicked, mode, itemstack, short1));\n",
        "        short short1 = playerIn.openContainer.getNextTransactionID(playerIn.inventory);\n        ItemStack itemstack = playerIn.openContainer.slotClick(slotId, mouseButtonClicked, mode, playerIn);\n        this.netClientHandler.addToSendQueue(new C0EPacketClickWindow(windowId, slotId, mouseButtonClicked, mode, itemstack, short1));\n        RmcTraceLogger.traceInventoryClick(windowId, slotId, mouseButtonClicked, mode, short1, itemstack);\n",
    )?;
    replace_once(
        &updated,
        "        this.syncCurrentPlayItem();\n        this.netClientHandler.addToSendQueue(new C07PacketPlayerDigging(C07PacketPlayerDigging.Action.RELEASE_USE_ITEM, BlockPos.ORIGIN, EnumFacing.DOWN));\n        playerIn.stopUsingItem();\n",
        "        this.syncCurrentPlayItem();\n        this.netClientHandler.addToSendQueue(new C07PacketPlayerDigging(C07PacketPlayerDigging.Action.RELEASE_USE_ITEM, BlockPos.ORIGIN, EnumFacing.DOWN));\n        RmcTraceLogger.traceCombatReleaseUseItem(playerIn.inventory.currentItem);\n        playerIn.stopUsingItem();\n",
    )
}

fn patch_net_handler_play_client(source: &str) -> Result<String, String> {
    let mut updated = ensure_import(
        source,
        "import net.minecraft.client.resources.I18n;\n",
        "import net.minecraft.client.rmc.RmcTraceLogger;\n",
    )?;
    updated = updated.replace(
        "        RmcTraceLogger.traceInventoryWindowItems(packetIn.func_148911_c(), packetIn.getItemStacks().size());\n\n",
        "",
    );
    updated = updated.replace(
        "packetIn.getItemStacks().size()",
        "packetIn.getItemStacks().length",
    );
    updated = updated.replace(
        "        RmcTraceLogger.traceInventoryWindowItems(packetIn.func_148911_c(), packetIn.getItemStacks().length);\n\n        RmcTraceLogger.traceInventoryWindowItems(packetIn.func_148911_c(), packetIn.getItemStacks().length);\n",
        "        RmcTraceLogger.traceInventoryWindowItems(packetIn.func_148911_c(), packetIn.getItemStacks().length);\n",
    );
    updated = replace_once(
        &updated,
        "        if (entity != null)\n        {\n            entity.setVelocity((double)packetIn.getMotionX() / 8000.0D, (double)packetIn.getMotionY() / 8000.0D, (double)packetIn.getMotionZ() / 8000.0D);\n        }\n",
        "        if (entity != null)\n        {\n            entity.setVelocity((double)packetIn.getMotionX() / 8000.0D, (double)packetIn.getMotionY() / 8000.0D, (double)packetIn.getMotionZ() / 8000.0D);\n            RmcTraceLogger.traceCombatVelocity(packetIn.getEntityID(), (double)packetIn.getMotionX() / 8000.0D, (double)packetIn.getMotionY() / 8000.0D, (double)packetIn.getMotionZ() / 8000.0D, entity == this.gameController.thePlayer);\n        }\n",
    )?;
    updated = replace_once(
        &updated,
        "        entityplayer.setPositionAndRotation(d0, d1, d2, f, f1);\n        this.netManager.sendPacket(new C03PacketPlayer.C06PacketPlayerPosLook(entityplayer.posX, entityplayer.getEntityBoundingBox().minY, entityplayer.posZ, entityplayer.rotationYaw, entityplayer.rotationPitch, false));\n",
        "        entityplayer.setPositionAndRotation(d0, d1, d2, f, f1);\n        RmcTraceLogger.traceMovementCorrection(d0, d1, d2, f, f1, packetIn.func_179834_f().toString());\n        this.netManager.sendPacket(new C03PacketPlayer.C06PacketPlayerPosLook(entityplayer.posX, entityplayer.getEntityBoundingBox().minY, entityplayer.posZ, entityplayer.rotationYaw, entityplayer.rotationPitch, false));\n",
    )?;
    updated = replace_once(
        &updated,
        "        this.gameController.thePlayer.setPlayerSPHealth(packetIn.getHealth());\n        this.gameController.thePlayer.getFoodStats().setFoodLevel(packetIn.getFoodLevel());\n        this.gameController.thePlayer.getFoodStats().setFoodSaturationLevel(packetIn.getSaturationLevel());\n",
        "        this.gameController.thePlayer.setPlayerSPHealth(packetIn.getHealth());\n        this.gameController.thePlayer.getFoodStats().setFoodLevel(packetIn.getFoodLevel());\n        this.gameController.thePlayer.getFoodStats().setFoodSaturationLevel(packetIn.getSaturationLevel());\n        RmcTraceLogger.traceCombatHealth(packetIn.getHealth(), packetIn.getFoodLevel(), packetIn.getSaturationLevel());\n",
    )?;
    updated = replace_once(
        &updated,
        "        else\n        {\n            ContainerLocalMenu containerlocalmenu = new ContainerLocalMenu(packetIn.getGuiId(), packetIn.getWindowTitle(), packetIn.getSlotCount());\n            entityplayersp.displayGUIChest(containerlocalmenu);\n            entityplayersp.openContainer.windowId = packetIn.getWindowId();\n        }\n",
        "        else\n        {\n            ContainerLocalMenu containerlocalmenu = new ContainerLocalMenu(packetIn.getGuiId(), packetIn.getWindowTitle(), packetIn.getSlotCount());\n            entityplayersp.displayGUIChest(containerlocalmenu);\n            entityplayersp.openContainer.windowId = packetIn.getWindowId();\n        }\n\n        RmcTraceLogger.traceInventoryOpenWindow(packetIn.getWindowId(), packetIn.getGuiId(), packetIn.getSlotCount(), packetIn.getWindowTitle().getUnformattedText());\n",
    )?;
    updated = replace_once(
        &updated,
        "        }\n    }\n\n    /**\n     * Verifies that the server and client are synchronized with respect to the inventory/container opened by the player\n",
        "        }\n\n        RmcTraceLogger.traceInventorySetSlot(packetIn.func_149175_c(), packetIn.func_149173_d(), packetIn.func_149174_e());\n    }\n\n    /**\n     * Verifies that the server and client are synchronized with respect to the inventory/container opened by the player\n",
    )?;
    updated = replace_once(
        &updated,
        "        if (container != null && !packetIn.func_148888_e())\n        {\n            this.addToSendQueue(new C0FPacketConfirmTransaction(packetIn.getWindowId(), packetIn.getActionNumber(), true));\n        }\n",
        "        boolean rmcTraceAckSent = container != null && !packetIn.func_148888_e();\n\n        if (rmcTraceAckSent)\n        {\n            this.addToSendQueue(new C0FPacketConfirmTransaction(packetIn.getWindowId(), packetIn.getActionNumber(), true));\n        }\n\n        RmcTraceLogger.traceInventoryConfirm(packetIn.getWindowId(), packetIn.getActionNumber(), packetIn.func_148888_e(), rmcTraceAckSent);\n",
    )?;
    updated = replace_once(
        &updated,
        "        else if (packetIn.func_148911_c() == entityplayer.openContainer.windowId)\n        {\n            entityplayer.openContainer.putStacksInSlots(packetIn.getItemStacks());\n        }\n",
        "        else if (packetIn.func_148911_c() == entityplayer.openContainer.windowId)\n        {\n            entityplayer.openContainer.putStacksInSlots(packetIn.getItemStacks());\n        }\n\n        RmcTraceLogger.traceInventoryWindowItems(packetIn.func_148911_c(), packetIn.getItemStacks().length);\n",
    )?;
    Ok(updated)
}

fn ensure_import(source: &str, anchor: &str, import_line: &str) -> Result<String, String> {
    if source.contains(import_line) {
        return Ok(source.to_owned());
    }

    let index = source
        .find(anchor)
        .ok_or_else(|| format!("import anchor not found: {}", anchor.trim()))?;
    let insert_at = index + anchor.len();
    let mut updated = source.to_owned();
    updated.insert_str(insert_at, import_line);
    Ok(updated)
}

fn replace_once(source: &str, from: &str, to: &str) -> Result<String, String> {
    if source.contains(to) {
        return Ok(source.to_owned());
    }

    let Some(index) = source.find(from) else {
        return Err(format!(
            "patch anchor not found: {}",
            summarize_anchor(from)
        ));
    };

    let mut updated = String::with_capacity(source.len() - from.len() + to.len());
    updated.push_str(&source[..index]);
    updated.push_str(to);
    updated.push_str(&source[index + from.len()..]);
    Ok(updated)
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    Ok(())
}

fn summarize_anchor(anchor: &str) -> String {
    anchor
        .lines()
        .next()
        .unwrap_or(anchor)
        .trim()
        .chars()
        .take(80)
        .collect()
}
