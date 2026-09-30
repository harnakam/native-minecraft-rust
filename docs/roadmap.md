# Roadmap

Implemented and exercised: Rust protocol/login transport; received chunk rendering; world-based movement collision and stepping; survival reach with wall occlusion; inventory pickup prediction and rejection rollback; strict comparison and source publication audit.

Required before claiming vanilla-compatible multiplayer:

1. Resolve strict Java/Rust movement and packet differences, including initial-position synchronization and tick alignment.
2. Complete complex block collision/selection shapes, liquids, ladders, effects and abilities.
3. Complete inventory slot restrictions, crafting, shift-click and drag behavior.
4. Complete entity, held-item and translucent rendering and gameplay feedback.
5. Expand differential scenarios and real online-mode multiplayer validation.
6. Validate the intended PvP server only after the compatibility gate legitimately passes.

Singleplayer is outside this implementation scope. Minecraft assets and reference sources must remain local and untracked.
