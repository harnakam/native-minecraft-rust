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

## Observed results (2026-09-30, Windows)

The official 1.8.9 server JAR was checked against its published object SHA1, then run locally after the user confirmed EULA acceptance. Offline compressed login, join, chunk reception, windowed rendering and a ten-second forward/sprint run succeeded. The window smoke received 49 chunks and advanced 118 ticks. The movement run advanced 200 ticks and received 77 chunks without a server disconnect.

The strict live comparison failed: packet 61 Rust / 52 Java records with 39 differences; movement 41 / 34 with 41 differences. Combat (3 / 3) and inventory (5 / 5) matched. These counts describe scripted coverage, not overall parity. Packet aliases, asynchronous ordering and initial teleport/tick alignment need investigation alongside actual behavior differences.

Old comparison logic discarded most records and vertical coordinates. It has been removed. All records are retained, empty traces fail, and old comparison reports cannot open the compatibility gate. The gate remains closed. Hypixel gameplay is unverified.

## Limits

Unit tests do not establish complete Minecraft compatibility. Additional fixtures must cover liquids, ladders, effects, complex block geometry, specialized inventory slots, respawn/dimension changes and real online multiplayer sessions. Authentication secrets must never appear in captures or commits.
