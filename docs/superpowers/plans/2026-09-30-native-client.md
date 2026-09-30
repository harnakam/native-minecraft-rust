# Native client implementation plan

**Goal:** Build and publish an independently implemented Rust protocol 47 client.
**Architecture:** Reuse authored Rust networking/rendering modules, replace
simulation rules against local MCP919, and keep assets outside source control.
**Spec:** ../specs/2026-09-30-native-client-design.md

## Execution

- [ ] Establish an offline workspace test baseline using the installed Rust toolchain.
- [ ] Add failing movement regressions: first-tick acceleration, jump order,
  small authoritative corrections, and replacement server velocity.
- [ ] Implement terrain AABB collision and connect received chunks to every live
  simulation tick; test walls, floors, slabs, voids, and sneaking at edges.
- [ ] Restrict survival entity targeting to vanilla reach and block occlusion.
- [ ] Exercise local protocol 47 login, chunks, movement, and transactions using
  a local server where available. Preserve the live-capture verification gate.
- [ ] Replace stale documentation with exact capabilities and verification evidence.
- [ ] Audit the publication allowlist, add CI, create clean public history, push
  with authenticated harnakam, and verify remote contents.

## Constraints

No MCP source, game assets, JARs, account files, tokens, captures, or build artifacts
in public history. No new production dependency without necessity. Never infer
Hypixel compatibility or ban safety from unit tests. Review tick batching, missing
chunks, rejected inventory clicks, and server corrections before publication.
