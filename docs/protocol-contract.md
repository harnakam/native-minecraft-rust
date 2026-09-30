# Protocol Contract

## Purpose

Freeze the protocol 47 wire contract before most of M1 is implemented.

The source of truth is the Rust contract modules:

- `crates/rmc-net/src/protocol/handshake.rs`
- `crates/rmc-net/src/protocol/login.rs`
- `crates/rmc-net/src/protocol/play_clientbound.rs`
- `crates/rmc-net/src/protocol/play_serverbound.rs`

This document explains the common framing rules and the packet set that is frozen in M0.

## Contract freeze requirements

Each frozen packet definition must lock:

- Packet ID
- Direction
- State
- Field order
- Field encoding
- VarInt usage
- Compression behavior
- Java reference class and handler

## Packet framing

### Before compression is enabled

- `packet_length`: VarInt
- `packet_id`: VarInt
- `packet_body`: packet-specific fields

### After compression is enabled

- `packet_length`: VarInt
- `data_length`: VarInt
- `packet_id`: VarInt
- `packet_body`: packet-specific fields

Rules:

- `Set Compression` is not compressed itself.
- After `Set Compression`, packets with uncompressed size below threshold use `data_length = 0`.
- Packets at or above threshold use zlib-compressed packet id + body.

## Encoding glossary

- `VarInt`: 1 to 5 bytes, 7-bit continuation format.
- `String(n)`: VarInt byte length + UTF-8 bytes, bounded by `n`.
- `ByteArray`: VarInt length + raw bytes.
- `UUID`: two big-endian `Long` values.
- `Position`: packed 64-bit block position from `BlockPos.toLong()`.
- `Chat`: JSON chat component serialized through `String(32767)`.
- `Enum VarInt`: enum ordinal encoded as VarInt.
- `Slot`: `Short item_id`, then if present `Byte count`, `Short damage`, `NBT`.
- `DataWatcher list`: watchable-object stream terminated by `0x7F`.
- `RemainingBytes(max 32767)`: consume the rest of the packet body after known fields.

## Handshake state

### Serverbound

| ID | Packet |
| --- | --- |
| `0x00` | Handshake Request |

## Login state

### Clientbound

| ID | Packet |
| --- | --- |
| `0x00` | Login Disconnect |
| `0x01` | Encryption Request |
| `0x02` | Login Success |
| `0x03` | Set Compression |

### Serverbound

| ID | Packet |
| --- | --- |
| `0x00` | Login Start |
| `0x01` | Encryption Response |

## Play state

### Clientbound

| ID | Packet |
| --- | --- |
| `0x00` | Keep Alive |
| `0x01` | Join Game |
| `0x02` | Chat Message |
| `0x06` | Update Health |
| `0x07` | Respawn |
| `0x08` | Player Position And Look |
| `0x0C` | Spawn Player |
| `0x12` | Entity Velocity |
| `0x21` | Chunk Data |
| `0x22` | Multi Block Change |
| `0x23` | Block Change |
| `0x26` | Map Chunk Bulk |
| `0x29` | Sound Effect |
| `0x2D` | Open Window |
| `0x2E` | Close Window |
| `0x2F` | Set Slot |
| `0x30` | Window Items |
| `0x32` | Confirm Transaction |
| `0x38` | Player List Item |
| `0x3B` | Scoreboard Objective |
| `0x3C` | Update Score |
| `0x3D` | Display Scoreboard |
| `0x3E` | Teams |
| `0x40` | Play Disconnect |

### Serverbound

| ID | Packet |
| --- | --- |
| `0x00` | Keep Alive |
| `0x01` | Chat Message |
| `0x02` | Use Entity |
| `0x03` | Player |
| `0x04` | Player Position |
| `0x05` | Player Look |
| `0x06` | Player Position And Look |
| `0x07` | Player Digging |
| `0x08` | Player Block Placement |
| `0x09` | Held Item Change |
| `0x0A` | Animation |
| `0x0B` | Entity Action |
| `0x0D` | Close Window |
| `0x0E` | Click Window |
| `0x0F` | Confirm Transaction |
| `0x15` | Client Settings |
| `0x17` | Custom Payload |

## M0 deliverable boundary

M0 freezes the schema, not the runtime codec.

That means:

- packet ids are fixed
- field order is fixed
- field encodings are fixed
- compression assumptions are fixed

The runtime parser and serializer work begins in M1 and must consume these frozen definitions instead of redefining them.

## Verification rule

Packet trace comparison starts in M1.

Reference flow:

1. Java client packet log
2. Rust client packet log
3. Diff packet id, packet name, field values, and send order

## MCP sources used for this freeze

- `MCP-919/src/minecraft/net/minecraft/network/EnumConnectionState.java`
- `MCP-919/src/minecraft/net/minecraft/network/PacketBuffer.java`
- Packet classes referenced directly in the Rust contract modules
