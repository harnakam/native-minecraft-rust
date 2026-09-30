//! Numeric block/tool behavior observed with the authored local MCP probe.
//! Regenerate with scripts/generate_mining_properties.py; contains no game assets.
pub fn block_properties(id: u16) -> Option<(f32, bool)> {
    match id {
        7 | 120 | 137 | 166 => Some((-1.0, false)),
        36 | 90 | 119 => Some((-1.0, true)),
        0 | 6 | 31 | 32 | 37 | 38 | 39 | 40 | 46 | 50 | 51 | 55 | 59 | 75 | 76 | 83 | 93 | 94
        | 104 | 105 | 111 | 115 | 131 | 132 | 140 | 141 | 142 | 149 | 150 | 165 | 175 => {
            Some((0.0, true))
        }
        78 => Some((0.1, false)),
        171 => Some((0.1, true)),
        80 => Some((0.2, false)),
        18 | 26 | 99 | 100 | 106 | 127 | 151 | 161 | 178 => Some((0.2, true)),
        20 | 89 | 95 | 102 | 123 | 124 | 160 | 169 => Some((0.3, true)),
        87 => Some((0.4, false)),
        65 | 81 => Some((0.4, true)),
        70 | 117 | 147 | 148 => Some((0.5, false)),
        3 | 12 | 29 | 33 | 34 | 69 | 72 | 77 | 79 | 88 | 92 | 143 | 170 | 174 => Some((0.5, true)),
        2 | 13 | 19 | 60 | 82 | 110 => Some((0.6, true)),
        27 | 28 | 66 | 157 => Some((0.7, true)),
        97 => Some((0.75, true)),
        24 | 128 | 155 | 156 | 179 | 180 => Some((0.8, false)),
        25 | 35 => Some((0.8, true)),
        63 | 68 | 86 | 91 | 103 | 144 | 176 | 177 => Some((1.0, true)),
        159 | 172 => Some((1.25, false)),
        1 | 98 | 109 | 168 => Some((1.5, false)),
        47 => Some((1.5, true)),
        4 | 43 | 44 | 45 | 48 | 67 | 108 | 112 | 113 | 114 | 118 | 139 | 181 | 182 => {
            Some((2.0, false))
        }
        5 | 17 | 53 | 84 | 85 | 107 | 125 | 126 | 134 | 135 | 136 | 162 | 163 | 164 | 183 | 184
        | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192 => Some((2.0, true)),
        54 | 58 | 146 => Some((2.5, true)),
        14 | 15 | 16 | 21 | 22 | 41 | 56 | 73 | 74 | 121 | 129 | 153 | 154 => Some((3.0, false)),
        64 | 96 | 122 | 138 | 193 | 194 | 195 | 196 | 197 => Some((3.0, true)),
        23 | 61 | 62 | 158 => Some((3.5, false)),
        30 => Some((4.0, false)),
        42 | 52 | 57 | 71 | 101 | 116 | 133 | 145 | 152 | 167 | 173 => Some((5.0, false)),
        130 => Some((22.5, false)),
        49 => Some((50.0, false)),
        8 | 9 | 10 | 11 => Some((100.0, true)),
        _ => None,
    }
}
pub fn tool_properties(block: u16, item: i16, hand: bool) -> (f32, bool) {
    match (item, block) {
        (256, 2 | 3 | 12 | 13 | 60 | 78 | 80 | 82 | 88 | 110) => (6.0, true),
        (257, 49) => (6.0, false),
        (
            257,
            1 | 4 | 7 | 14 | 15 | 16 | 21 | 22 | 23 | 24 | 27 | 28 | 41 | 42 | 43 | 44 | 45 | 48
            | 52 | 56 | 57 | 61 | 62 | 66 | 67 | 70 | 71 | 73 | 74 | 79 | 87 | 98 | 101 | 108 | 109
            | 112 | 113 | 114 | 116 | 117 | 118 | 120 | 121 | 128 | 129 | 130 | 133 | 137 | 139
            | 145 | 147 | 148 | 152 | 153 | 154 | 155 | 156 | 157 | 158 | 159 | 167 | 168 | 172
            | 173 | 174 | 179 | 180 | 181 | 182,
        ) => (6.0, true),
        (
            258,
            5 | 6 | 17 | 25 | 31 | 32 | 37 | 38 | 39 | 40 | 47 | 53 | 54 | 58 | 59 | 63 | 64 | 65
            | 68 | 72 | 83 | 84 | 85 | 86 | 91 | 96 | 99 | 100 | 103 | 104 | 105 | 106 | 107 | 111
            | 115 | 125 | 126 | 127 | 134 | 135 | 136 | 141 | 142 | 146 | 151 | 162 | 163 | 164
            | 175 | 176 | 177 | 178 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192
            | 193 | 194 | 195 | 196 | 197,
        ) => (6.0, true),
        (
            267,
            6 | 18 | 31 | 32 | 37 | 38 | 39 | 40 | 59 | 83 | 86 | 91 | 103 | 104 | 105 | 106 | 111
            | 115 | 127 | 141 | 142 | 161 | 175,
        ) => (1.5, true),
        (267, 30) => (15.0, true),
        (
            268,
            6 | 18 | 31 | 32 | 37 | 38 | 39 | 40 | 59 | 83 | 86 | 91 | 103 | 104 | 105 | 106 | 111
            | 115 | 127 | 141 | 142 | 161 | 175,
        ) => (1.5, true),
        (268, 30) => (15.0, true),
        (269, 2 | 3 | 12 | 13 | 60 | 78 | 80 | 82 | 88 | 110) => (2.0, true),
        (270, 14 | 15 | 21 | 22 | 41 | 42 | 49 | 56 | 57 | 73 | 74 | 129 | 133) => (2.0, false),
        (
            270,
            1 | 4 | 7 | 16 | 23 | 24 | 27 | 28 | 43 | 44 | 45 | 48 | 52 | 61 | 62 | 66 | 67 | 70
            | 71 | 79 | 87 | 98 | 101 | 108 | 109 | 112 | 113 | 114 | 116 | 117 | 118 | 120 | 121
            | 128 | 130 | 137 | 139 | 145 | 147 | 148 | 152 | 153 | 154 | 155 | 156 | 157 | 158
            | 159 | 167 | 168 | 172 | 173 | 174 | 179 | 180 | 181 | 182,
        ) => (2.0, true),
        (
            271,
            5 | 6 | 17 | 25 | 31 | 32 | 37 | 38 | 39 | 40 | 47 | 53 | 54 | 58 | 59 | 63 | 64 | 65
            | 68 | 72 | 83 | 84 | 85 | 86 | 91 | 96 | 99 | 100 | 103 | 104 | 105 | 106 | 107 | 111
            | 115 | 125 | 126 | 127 | 134 | 135 | 136 | 141 | 142 | 146 | 151 | 162 | 163 | 164
            | 175 | 176 | 177 | 178 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192
            | 193 | 194 | 195 | 196 | 197,
        ) => (2.0, true),
        (
            272,
            6 | 18 | 31 | 32 | 37 | 38 | 39 | 40 | 59 | 83 | 86 | 91 | 103 | 104 | 105 | 106 | 111
            | 115 | 127 | 141 | 142 | 161 | 175,
        ) => (1.5, true),
        (272, 30) => (15.0, true),
        (273, 2 | 3 | 12 | 13 | 60 | 78 | 80 | 82 | 88 | 110) => (4.0, true),
        (274, 14 | 41 | 49 | 56 | 57 | 73 | 74 | 129 | 133) => (4.0, false),
        (
            274,
            1 | 4 | 7 | 15 | 16 | 21 | 22 | 23 | 24 | 27 | 28 | 42 | 43 | 44 | 45 | 48 | 52 | 61
            | 62 | 66 | 67 | 70 | 71 | 79 | 87 | 98 | 101 | 108 | 109 | 112 | 113 | 114 | 116 | 117
            | 118 | 120 | 121 | 128 | 130 | 137 | 139 | 145 | 147 | 148 | 152 | 153 | 154 | 155
            | 156 | 157 | 158 | 159 | 167 | 168 | 172 | 173 | 174 | 179 | 180 | 181 | 182,
        ) => (4.0, true),
        (
            275,
            5 | 6 | 17 | 25 | 31 | 32 | 37 | 38 | 39 | 40 | 47 | 53 | 54 | 58 | 59 | 63 | 64 | 65
            | 68 | 72 | 83 | 84 | 85 | 86 | 91 | 96 | 99 | 100 | 103 | 104 | 105 | 106 | 107 | 111
            | 115 | 125 | 126 | 127 | 134 | 135 | 136 | 141 | 142 | 146 | 151 | 162 | 163 | 164
            | 175 | 176 | 177 | 178 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192
            | 193 | 194 | 195 | 196 | 197,
        ) => (4.0, true),
        (
            276,
            6 | 18 | 31 | 32 | 37 | 38 | 39 | 40 | 59 | 83 | 86 | 91 | 103 | 104 | 105 | 106 | 111
            | 115 | 127 | 141 | 142 | 161 | 175,
        ) => (1.5, true),
        (276, 30) => (15.0, true),
        (277, 2 | 3 | 12 | 13 | 60 | 78 | 80 | 82 | 88 | 110) => (8.0, true),
        (
            278,
            1 | 4 | 7 | 14 | 15 | 16 | 21 | 22 | 23 | 24 | 27 | 28 | 41 | 42 | 43 | 44 | 45 | 48
            | 49 | 52 | 56 | 57 | 61 | 62 | 66 | 67 | 70 | 71 | 73 | 74 | 79 | 87 | 98 | 101 | 108
            | 109 | 112 | 113 | 114 | 116 | 117 | 118 | 120 | 121 | 128 | 129 | 130 | 133 | 137
            | 139 | 145 | 147 | 148 | 152 | 153 | 154 | 155 | 156 | 157 | 158 | 159 | 167 | 168
            | 172 | 173 | 174 | 179 | 180 | 181 | 182,
        ) => (8.0, true),
        (
            279,
            5 | 6 | 17 | 25 | 31 | 32 | 37 | 38 | 39 | 40 | 47 | 53 | 54 | 58 | 59 | 63 | 64 | 65
            | 68 | 72 | 83 | 84 | 85 | 86 | 91 | 96 | 99 | 100 | 103 | 104 | 105 | 106 | 107 | 111
            | 115 | 125 | 126 | 127 | 134 | 135 | 136 | 141 | 142 | 146 | 151 | 162 | 163 | 164
            | 175 | 176 | 177 | 178 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192
            | 193 | 194 | 195 | 196 | 197,
        ) => (8.0, true),
        (
            283,
            6 | 18 | 31 | 32 | 37 | 38 | 39 | 40 | 59 | 83 | 86 | 91 | 103 | 104 | 105 | 106 | 111
            | 115 | 127 | 141 | 142 | 161 | 175,
        ) => (1.5, true),
        (283, 30) => (15.0, true),
        (284, 2 | 3 | 12 | 13 | 60 | 78 | 80 | 82 | 88 | 110) => (12.0, true),
        (285, 14 | 15 | 21 | 22 | 41 | 42 | 49 | 56 | 57 | 73 | 74 | 129 | 133) => (12.0, false),
        (
            285,
            1 | 4 | 7 | 16 | 23 | 24 | 27 | 28 | 43 | 44 | 45 | 48 | 52 | 61 | 62 | 66 | 67 | 70
            | 71 | 79 | 87 | 98 | 101 | 108 | 109 | 112 | 113 | 114 | 116 | 117 | 118 | 120 | 121
            | 128 | 130 | 137 | 139 | 145 | 147 | 148 | 152 | 153 | 154 | 155 | 156 | 157 | 158
            | 159 | 167 | 168 | 172 | 173 | 174 | 179 | 180 | 181 | 182,
        ) => (12.0, true),
        (
            286,
            5 | 6 | 17 | 25 | 31 | 32 | 37 | 38 | 39 | 40 | 47 | 53 | 54 | 58 | 59 | 63 | 64 | 65
            | 68 | 72 | 83 | 84 | 85 | 86 | 91 | 96 | 99 | 100 | 103 | 104 | 105 | 106 | 107 | 111
            | 115 | 125 | 126 | 127 | 134 | 135 | 136 | 141 | 142 | 146 | 151 | 162 | 163 | 164
            | 175 | 176 | 177 | 178 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192
            | 193 | 194 | 195 | 196 | 197,
        ) => (12.0, true),
        (359, 35) => (5.0, true),
        (359, 18 | 30 | 161) => (15.0, true),
        _ => (1.0, hand),
    }
}
pub fn block_name(id: u16) -> Option<&'static str> {
    match id {
        0 => Some("minecraft:air"),
        1 => Some("minecraft:stone"),
        2 => Some("minecraft:grass"),
        3 => Some("minecraft:dirt"),
        4 => Some("minecraft:cobblestone"),
        5 => Some("minecraft:planks"),
        6 => Some("minecraft:sapling"),
        7 => Some("minecraft:bedrock"),
        8 => Some("minecraft:flowing_water"),
        9 => Some("minecraft:water"),
        10 => Some("minecraft:flowing_lava"),
        11 => Some("minecraft:lava"),
        12 => Some("minecraft:sand"),
        13 => Some("minecraft:gravel"),
        14 => Some("minecraft:gold_ore"),
        15 => Some("minecraft:iron_ore"),
        16 => Some("minecraft:coal_ore"),
        17 => Some("minecraft:log"),
        18 => Some("minecraft:leaves"),
        19 => Some("minecraft:sponge"),
        20 => Some("minecraft:glass"),
        21 => Some("minecraft:lapis_ore"),
        22 => Some("minecraft:lapis_block"),
        23 => Some("minecraft:dispenser"),
        24 => Some("minecraft:sandstone"),
        25 => Some("minecraft:noteblock"),
        26 => Some("minecraft:bed"),
        27 => Some("minecraft:golden_rail"),
        28 => Some("minecraft:detector_rail"),
        29 => Some("minecraft:sticky_piston"),
        30 => Some("minecraft:web"),
        31 => Some("minecraft:tallgrass"),
        32 => Some("minecraft:deadbush"),
        33 => Some("minecraft:piston"),
        34 => Some("minecraft:piston_head"),
        35 => Some("minecraft:wool"),
        36 => Some("minecraft:piston_extension"),
        37 => Some("minecraft:yellow_flower"),
        38 => Some("minecraft:red_flower"),
        39 => Some("minecraft:brown_mushroom"),
        40 => Some("minecraft:red_mushroom"),
        41 => Some("minecraft:gold_block"),
        42 => Some("minecraft:iron_block"),
        43 => Some("minecraft:double_stone_slab"),
        44 => Some("minecraft:stone_slab"),
        45 => Some("minecraft:brick_block"),
        46 => Some("minecraft:tnt"),
        47 => Some("minecraft:bookshelf"),
        48 => Some("minecraft:mossy_cobblestone"),
        49 => Some("minecraft:obsidian"),
        50 => Some("minecraft:torch"),
        51 => Some("minecraft:fire"),
        52 => Some("minecraft:mob_spawner"),
        53 => Some("minecraft:oak_stairs"),
        54 => Some("minecraft:chest"),
        55 => Some("minecraft:redstone_wire"),
        56 => Some("minecraft:diamond_ore"),
        57 => Some("minecraft:diamond_block"),
        58 => Some("minecraft:crafting_table"),
        59 => Some("minecraft:wheat"),
        60 => Some("minecraft:farmland"),
        61 => Some("minecraft:furnace"),
        62 => Some("minecraft:lit_furnace"),
        63 => Some("minecraft:standing_sign"),
        64 => Some("minecraft:wooden_door"),
        65 => Some("minecraft:ladder"),
        66 => Some("minecraft:rail"),
        67 => Some("minecraft:stone_stairs"),
        68 => Some("minecraft:wall_sign"),
        69 => Some("minecraft:lever"),
        70 => Some("minecraft:stone_pressure_plate"),
        71 => Some("minecraft:iron_door"),
        72 => Some("minecraft:wooden_pressure_plate"),
        73 => Some("minecraft:redstone_ore"),
        74 => Some("minecraft:lit_redstone_ore"),
        75 => Some("minecraft:unlit_redstone_torch"),
        76 => Some("minecraft:redstone_torch"),
        77 => Some("minecraft:stone_button"),
        78 => Some("minecraft:snow_layer"),
        79 => Some("minecraft:ice"),
        80 => Some("minecraft:snow"),
        81 => Some("minecraft:cactus"),
        82 => Some("minecraft:clay"),
        83 => Some("minecraft:reeds"),
        84 => Some("minecraft:jukebox"),
        85 => Some("minecraft:fence"),
        86 => Some("minecraft:pumpkin"),
        87 => Some("minecraft:netherrack"),
        88 => Some("minecraft:soul_sand"),
        89 => Some("minecraft:glowstone"),
        90 => Some("minecraft:portal"),
        91 => Some("minecraft:lit_pumpkin"),
        92 => Some("minecraft:cake"),
        93 => Some("minecraft:unpowered_repeater"),
        94 => Some("minecraft:powered_repeater"),
        95 => Some("minecraft:stained_glass"),
        96 => Some("minecraft:trapdoor"),
        97 => Some("minecraft:monster_egg"),
        98 => Some("minecraft:stonebrick"),
        99 => Some("minecraft:brown_mushroom_block"),
        100 => Some("minecraft:red_mushroom_block"),
        101 => Some("minecraft:iron_bars"),
        102 => Some("minecraft:glass_pane"),
        103 => Some("minecraft:melon_block"),
        104 => Some("minecraft:pumpkin_stem"),
        105 => Some("minecraft:melon_stem"),
        106 => Some("minecraft:vine"),
        107 => Some("minecraft:fence_gate"),
        108 => Some("minecraft:brick_stairs"),
        109 => Some("minecraft:stone_brick_stairs"),
        110 => Some("minecraft:mycelium"),
        111 => Some("minecraft:waterlily"),
        112 => Some("minecraft:nether_brick"),
        113 => Some("minecraft:nether_brick_fence"),
        114 => Some("minecraft:nether_brick_stairs"),
        115 => Some("minecraft:nether_wart"),
        116 => Some("minecraft:enchanting_table"),
        117 => Some("minecraft:brewing_stand"),
        118 => Some("minecraft:cauldron"),
        119 => Some("minecraft:end_portal"),
        120 => Some("minecraft:end_portal_frame"),
        121 => Some("minecraft:end_stone"),
        122 => Some("minecraft:dragon_egg"),
        123 => Some("minecraft:redstone_lamp"),
        124 => Some("minecraft:lit_redstone_lamp"),
        125 => Some("minecraft:double_wooden_slab"),
        126 => Some("minecraft:wooden_slab"),
        127 => Some("minecraft:cocoa"),
        128 => Some("minecraft:sandstone_stairs"),
        129 => Some("minecraft:emerald_ore"),
        130 => Some("minecraft:ender_chest"),
        131 => Some("minecraft:tripwire_hook"),
        132 => Some("minecraft:tripwire"),
        133 => Some("minecraft:emerald_block"),
        134 => Some("minecraft:spruce_stairs"),
        135 => Some("minecraft:birch_stairs"),
        136 => Some("minecraft:jungle_stairs"),
        137 => Some("minecraft:command_block"),
        138 => Some("minecraft:beacon"),
        139 => Some("minecraft:cobblestone_wall"),
        140 => Some("minecraft:flower_pot"),
        141 => Some("minecraft:carrots"),
        142 => Some("minecraft:potatoes"),
        143 => Some("minecraft:wooden_button"),
        144 => Some("minecraft:skull"),
        145 => Some("minecraft:anvil"),
        146 => Some("minecraft:trapped_chest"),
        147 => Some("minecraft:light_weighted_pressure_plate"),
        148 => Some("minecraft:heavy_weighted_pressure_plate"),
        149 => Some("minecraft:unpowered_comparator"),
        150 => Some("minecraft:powered_comparator"),
        151 => Some("minecraft:daylight_detector"),
        152 => Some("minecraft:redstone_block"),
        153 => Some("minecraft:quartz_ore"),
        154 => Some("minecraft:hopper"),
        155 => Some("minecraft:quartz_block"),
        156 => Some("minecraft:quartz_stairs"),
        157 => Some("minecraft:activator_rail"),
        158 => Some("minecraft:dropper"),
        159 => Some("minecraft:stained_hardened_clay"),
        160 => Some("minecraft:stained_glass_pane"),
        161 => Some("minecraft:leaves2"),
        162 => Some("minecraft:log2"),
        163 => Some("minecraft:acacia_stairs"),
        164 => Some("minecraft:dark_oak_stairs"),
        165 => Some("minecraft:slime"),
        166 => Some("minecraft:barrier"),
        167 => Some("minecraft:iron_trapdoor"),
        168 => Some("minecraft:prismarine"),
        169 => Some("minecraft:sea_lantern"),
        170 => Some("minecraft:hay_block"),
        171 => Some("minecraft:carpet"),
        172 => Some("minecraft:hardened_clay"),
        173 => Some("minecraft:coal_block"),
        174 => Some("minecraft:packed_ice"),
        175 => Some("minecraft:double_plant"),
        176 => Some("minecraft:standing_banner"),
        177 => Some("minecraft:wall_banner"),
        178 => Some("minecraft:daylight_detector_inverted"),
        179 => Some("minecraft:red_sandstone"),
        180 => Some("minecraft:red_sandstone_stairs"),
        181 => Some("minecraft:double_stone_slab2"),
        182 => Some("minecraft:stone_slab2"),
        183 => Some("minecraft:spruce_fence_gate"),
        184 => Some("minecraft:birch_fence_gate"),
        185 => Some("minecraft:jungle_fence_gate"),
        186 => Some("minecraft:dark_oak_fence_gate"),
        187 => Some("minecraft:acacia_fence_gate"),
        188 => Some("minecraft:spruce_fence"),
        189 => Some("minecraft:birch_fence"),
        190 => Some("minecraft:jungle_fence"),
        191 => Some("minecraft:dark_oak_fence"),
        192 => Some("minecraft:acacia_fence"),
        193 => Some("minecraft:spruce_door"),
        194 => Some("minecraft:birch_door"),
        195 => Some("minecraft:jungle_door"),
        196 => Some("minecraft:acacia_door"),
        197 => Some("minecraft:dark_oak_door"),
        _ => None,
    }
}
