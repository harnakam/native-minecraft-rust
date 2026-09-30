# Native Minecraft 1.8.9 Rust client

Create `harnakam/native-minecraft-rust` as a public source repository titled
**A Native Minecraft 1.8.9 Client Written in Rust**. The runtime remains Rust;
MCP919 is a local behavioral reference, never a distributed dependency.
Reuse independently authored Rust modules from the previous repository snapshot
and replace inaccurate behavior against the MCP reference.

Protocol 47 multiplayer is the target. First validate with a local vanilla 1.8.9
server, then a legitimate online account on Hypixel. Singleplayer, Forge, and
complete graphical parity are outside this build. Do not describe sample traces
or a successful login as proof of gameplay parity.

Correct movement tick order, terrain collision, teleport corrections, velocity
replacement, attack reach/occlusion, and transaction handling. Preserve the existing
network/game/world/render/UI boundaries. Add regression tests from MCP behavior
before changing each rule. Assets may be imported from a user's installed game
into ignored local storage; no Mojang assets or decompiled classes are published.

Publish a clean initial history containing an explicit allowlist of source,
manifests, documentation, and independent verification tools. Audit for binary
assets and secrets before pushing; add automated CI and a publication audit.
Document any unsupported gameplay and unavailable live verification explicitly.
