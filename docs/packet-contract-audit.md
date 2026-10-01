# Protocol 47受信パケット契約監査

MCP919 EnumConnectionState.PLAYの登録順（内部クラスを含み、Javaの内部クラス区切り表記を正規化）とRust PACKETSを2026-10-01に照合した。
契約登録はdecode・状態反映・描画・タイミング互換の完了を意味しない。
未登録は実装対象として残し、登録済みも効果を個別監査する。

全74パケット中、IDとJavaクラス名が一致する契約は50件。

| ID | Javaクラス | 状態 |
| --- | --- | --- |
| 0x00 | S00PacketKeepAlive | 登録あり・効果は要監査 |
| 0x01 | S01PacketJoinGame | 登録あり・効果は要監査 |
| 0x02 | S02PacketChat | 登録あり・効果は要監査 |
| 0x03 | S03PacketTimeUpdate | 登録あり・効果は要監査 |
| 0x04 | S04PacketEntityEquipment | 登録あり・効果は要監査 |
| 0x05 | S05PacketSpawnPosition | 登録あり・両スポーン状態のテストあり |
| 0x06 | S06PacketUpdateHealth | 登録あり・効果は要監査 |
| 0x07 | S07PacketRespawn | 登録あり・効果は要監査 |
| 0x08 | S08PacketPlayerPosLook | 登録あり・効果は要監査 |
| 0x09 | S09PacketHeldItemChange | 登録あり・効果は要監査 |
| 0x0A | S0APacketUseBed | 未登録 |
| 0x0B | S0BPacketAnimation | 未登録 |
| 0x0C | S0CPacketSpawnPlayer | 登録あり・効果は要監査 |
| 0x0D | S0DPacketCollectItem | 未登録 |
| 0x0E | S0EPacketSpawnObject | 未登録 |
| 0x0F | S0FPacketSpawnMob | 未登録 |
| 0x10 | S10PacketSpawnPainting | 未登録 |
| 0x11 | S11PacketSpawnExperienceOrb | 未登録 |
| 0x12 | S12PacketEntityVelocity | 登録あり・効果は要監査 |
| 0x13 | S13PacketDestroyEntities | 登録あり・効果は要監査 |
| 0x14 | S14PacketEntity | 未登録 |
| 0x15 | S15PacketEntityRelMove | 登録あり・内部クラス表記（効果は要監査） |
| 0x16 | S16PacketEntityLook | 登録あり・内部クラス表記（効果は要監査） |
| 0x17 | S17PacketEntityLookMove | 登録あり・内部クラス表記（効果は要監査） |
| 0x18 | S18PacketEntityTeleport | 登録あり・効果は要監査 |
| 0x19 | S19PacketEntityHeadLook | 登録あり・効果は要監査 |
| 0x1A | S19PacketEntityStatus | 未登録 |
| 0x1B | S1BPacketEntityAttach | 未登録 |
| 0x1C | S1CPacketEntityMetadata | 登録あり・効果は要監査 |
| 0x1D | S1DPacketEntityEffect | 登録あり・効果は要監査 |
| 0x1E | S1EPacketRemoveEntityEffect | 登録あり・効果は要監査 |
| 0x1F | S1FPacketSetExperience | 登録あり・効果は要監査 |
| 0x20 | S20PacketEntityProperties | 登録あり・効果は要監査 |
| 0x21 | S21PacketChunkData | 登録あり・効果は要監査 |
| 0x22 | S22PacketMultiBlockChange | 登録あり・効果は要監査 |
| 0x23 | S23PacketBlockChange | 登録あり・効果は要監査 |
| 0x24 | S24PacketBlockAction | 未登録 |
| 0x25 | S25PacketBlockBreakAnim | 未登録 |
| 0x26 | S26PacketMapChunkBulk | 登録あり・効果は要監査 |
| 0x27 | S27PacketExplosion | 登録あり・効果は要監査 |
| 0x28 | S28PacketEffect | 未登録 |
| 0x29 | S29PacketSoundEffect | 登録あり・効果は要監査 |
| 0x2A | S2APacketParticles | 未登録 |
| 0x2B | S2BPacketChangeGameState | 登録あり・効果は要監査 |
| 0x2C | S2CPacketSpawnGlobalEntity | 未登録 |
| 0x2D | S2DPacketOpenWindow | 登録あり・効果は要監査 |
| 0x2E | S2EPacketCloseWindow | 登録あり・効果は要監査 |
| 0x2F | S2FPacketSetSlot | 登録あり・効果は要監査 |
| 0x30 | S30PacketWindowItems | 登録あり・効果は要監査 |
| 0x31 | S31PacketWindowProperty | 登録あり・効果は要監査 |
| 0x32 | S32PacketConfirmTransaction | 登録あり・効果は要監査 |
| 0x33 | S33PacketUpdateSign | 未登録 |
| 0x34 | S34PacketMaps | 登録あり・効果は要監査 |
| 0x35 | S35PacketUpdateTileEntity | 未登録 |
| 0x36 | S36PacketSignEditorOpen | 未登録 |
| 0x37 | S37PacketStatistics | 登録あり・効果は要監査 |
| 0x38 | S38PacketPlayerListItem | 登録あり・効果は要監査 |
| 0x39 | S39PacketPlayerAbilities | 登録あり・効果は要監査 |
| 0x3A | S3APacketTabComplete | 未登録 |
| 0x3B | S3BPacketScoreboardObjective | 登録あり・効果は要監査 |
| 0x3C | S3CPacketUpdateScore | 登録あり・効果は要監査 |
| 0x3D | S3DPacketDisplayScoreboard | 登録あり・効果は要監査 |
| 0x3E | S3EPacketTeams | 登録あり・効果は要監査 |
| 0x3F | S3FPacketCustomPayload | 未登録 |
| 0x40 | S40PacketDisconnect | 登録あり・効果は要監査 |
| 0x41 | S41PacketServerDifficulty | 登録あり・効果は要監査 |
| 0x42 | S42PacketCombatEvent | 未登録 |
| 0x43 | S43PacketCamera | 未登録 |
| 0x44 | S44PacketWorldBorder | 登録あり・効果は要監査 |
| 0x45 | S45PacketTitle | 登録あり・効果は要監査 |
| 0x46 | S46PacketSetCompressionLevel | 未登録 |
| 0x47 | S47PacketPlayerListHeaderFooter | 登録あり・効果は要監査 |
| 0x48 | S48PacketResourcePackSend | 登録あり・効果は要監査 |
| 0x49 | S49PacketUpdateEntityNBT | 未登録 |
