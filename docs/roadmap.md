# Roadmap

Implemented and exercised: Rust protocol/login transport; received chunk rendering; world-based movement collision and stepping; survival reach with wall occlusion; inventory pickup prediction and rejection rollback; strict comparison and source publication audit.

Required before claiming vanilla-compatible multiplayer:

1. Expand strict Java/Rust scenarios beyond the currently passing packet, movement, combat and inventory captures.
2. Complete special mining interactions and tick batching, connected collision/selection shapes, fluid/enchantment edge cases, effects and local ability controls.
3. Complete inventory slot restrictions, crafting, shift-click and drag behavior.
4. Complete entity, held-item and translucent rendering and gameplay feedback.
5. Implement remaining protocol handlers, cross-dimension respawn and riding behavior; expand real online-mode multiplayer validation.
6. Validate the intended PvP server only after the compatibility gate legitimately passes.

Singleplayer is outside this implementation scope. Minecraft assets and reference sources must remain local and untracked.
