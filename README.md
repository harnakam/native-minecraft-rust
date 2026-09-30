# A Native Minecraft 1.8.9 Client Written in Rust

A native Rust multiplayer client targeting Minecraft 1.8.9 (protocol 47). This is an independent implementation informed by local MCP919 behavioral analysis. It is **not yet a complete vanilla-compatible client**. Singleplayer is outside the current scope.

## Run

Install stable Rust, then:

```sh
cargo build -p rmc-client --locked
cargo run -p rmc-client -- play --server localhost:25565 --offline --offline-name Player
```

Offline login requires an offline-mode server. For online login, the window supports locally installed launcher accounts and browser-based Microsoft authentication. Never commit account files or tokens.

The windowed client receives chunks, renders terrain, simulates movement against received block geometry, and provides chat, hotbar, entity interaction, block mining, death/respawn controls and container pickup handling. Local assets are imported at runtime from an installed 1.8.9 JAR or a local MCP919 installation into ignored `local_assets/`. No Minecraft assets are distributed here.

## Compatibility status

Verified on Windows against a local official 1.8.9 server: compressed login, world join, 49 received chunks, windowed rendering and sustained forward/sprint movement. Unit tests cover protocol framing, partial writes with encryption, movement rules, terrain collision, interaction reach, inventory rollback and comparison integrity.

The strict live Java/Rust comparison passes the current packet, movement, combat and inventory scenarios. Local differential probes also match 1,360 travel ticks, 3,039 valid single-block collision shapes, 91,170 selection rays, 26,136 mining-speed values and 140 mining-controller ticks. A local official server confirms stone mining and respawn. These are limited scenarios: neighboring blocks, all gameplay actions and online PvP are not comprehensively verified. Hypixel gameplay has not been verified. Do not interpret these results as full behavior parity.

Remaining work includes special mining cases, connected collision/selection, fluid/enchantment edge cases, full inventory crafting/shift/drag rules, entity/item/model rendering, remaining protocol handlers and broader multiplayer interaction tests. See [verification](docs/verification.md) and [roadmap](docs/roadmap.md).

## Source layout

- `rmc-client`: window, CLI, live runtime and verification orchestration.
- `rmc-net`: protocol 47, authentication, compression and encrypted transport.
- `rmc-game`: movement, combat and inventory state.
- `rmc-world`: received chunks, entities, collision geometry and raycasting.
- `rmc-render` / `rmc-ui`: rendering and client interfaces.
- `rmc-java`: authored local MCP instrumentation tools; Java is not required for the Rust runtime.

## Source-only publication

MCP919, decompiled Minecraft classes, JARs, resource packs, textures, fonts, sounds, server worlds and credentials are excluded. The publication audit checks tracked source files. Minecraft is a trademark of Mojang; this project is not affiliated with Mojang or Microsoft.

```sh
python scripts/audit_publication.py
cargo fmt --all -- --check
cargo test --workspace --locked
```
