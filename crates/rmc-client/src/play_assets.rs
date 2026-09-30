use image::ImageReader;
use rmc_render::ChunkFace;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const VANILLA_VERSION: &str = "1.8.9";

#[derive(Clone, Debug)]
pub struct ImageAsset {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl ImageAsset {
    pub fn load(path: &Path) -> Option<Self> {
        let image = ImageReader::open(path).ok()?.decode().ok()?.to_rgba8();
        Some(Self {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn sample_repeat(&self, u: f32, v: f32) -> [u8; 4] {
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);
        let x = ((u * self.width as f32).floor() as u32).min(self.width.saturating_sub(1));
        let y = ((v * self.height as f32).floor() as u32).min(self.height.saturating_sub(1));
        self.pixel(x, y)
    }

    pub fn sample_linear_repeat(&self, u: f32, v: f32) -> [f32; 4] {
        let x = u.rem_euclid(1.0) * self.width as f32 - 0.5;
        let y = v.rem_euclid(1.0) * self.height as f32 - 0.5;
        let x0 = x.floor() as i64;
        let y0 = y.floor() as i64;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let sample = |dx: i64, dy: i64| {
            self.pixel(
                (x0 + dx).rem_euclid(i64::from(self.width)) as u32,
                (y0 + dy).rem_euclid(i64::from(self.height)) as u32,
            )
        };
        let [a, b, c, d] = [sample(0, 0), sample(1, 0), sample(0, 1), sample(1, 1)];
        std::array::from_fn(|channel| {
            let top = f32::from(a[channel]) * (1.0 - fx) + f32::from(b[channel]) * fx;
            let bottom = f32::from(c[channel]) * (1.0 - fx) + f32::from(d[channel]) * fx;
            (top * (1.0 - fy) + bottom * fy) / 255.0
        })
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ]
    }
}

#[derive(Clone, Debug)]
pub struct UnicodeFontAsset {
    pub glyph_widths: Vec<u8>,
    root: PathBuf,
    pages: std::sync::Arc<std::sync::Mutex<BTreeMap<u8, Option<ImageAsset>>>>,
}

impl UnicodeFontAsset {
    fn load(root: &Path) -> Option<Self> {
        let glyph_widths = fs::read(root.join("assets/minecraft/font/glyph_sizes.bin")).ok()?;
        if glyph_widths.len() != 65536 {
            return None;
        }
        Some(Self {
            glyph_widths,
            root: root.to_owned(),
            pages: Default::default(),
        })
    }
    pub fn with_page<T>(&self, ch: char, draw: impl FnOnce(&ImageAsset) -> T) -> Option<T> {
        let page = u8::try_from(ch as u32 >> 8).ok()?;
        let mut pages = self.pages.lock().ok()?;
        let image = pages.entry(page).or_insert_with(|| {
            ImageAsset::load(&self.root.join(format!(
                "assets/minecraft/textures/font/unicode_page_{page:02x}.png"
            )))
        });
        let image = image.as_ref()?;
        if image.width() != 256 || image.height() != 256 {
            return None;
        }
        Some(draw(image))
    }
}

#[derive(Default)]
pub struct GameAssets {
    pub vanilla_root: Option<PathBuf>,
    pub translations: BTreeMap<String, String>,
    pub widgets: Option<ImageAsset>,
    pub icons: Option<ImageAsset>,
    pub ascii_font: Option<ImageAsset>,
    pub unicode_font: Option<UnicodeFontAsset>,
    pub vignette: Option<ImageAsset>,
    block_textures: BTreeMap<String, ImageAsset>,
}

impl GameAssets {
    pub fn load() -> (Self, Option<String>) {
        let mut assets = Self::default();
        let mut notice = None;

        match ensure_vanilla_assets() {
            Ok((root, extracted)) => {
                assets.vanilla_root = Some(root.clone());
                if extracted {
                    notice =
                        Some("Imported local vanilla textures from Minecraft 1.8.9 jar".to_owned());
                }

                let language_path = root.join("assets/minecraft/lang/en_US.lang");
                match fs::read_to_string(&language_path) {
                    Ok(text) => assets.translations = parse_language_table(&text),
                    Err(error) => notice = Some(format!("Vanilla language unavailable: {error}")),
                }
                let texture_root = root.join("assets").join("minecraft").join("textures");
                assets.widgets = ImageAsset::load(&texture_root.join("gui").join("widgets.png"));
                assets.icons = ImageAsset::load(&texture_root.join("gui").join("icons.png"));
                assets.ascii_font = ImageAsset::load(&texture_root.join("font").join("ascii.png"));
                assets.unicode_font = UnicodeFontAsset::load(&root);
                assets.vignette = ImageAsset::load(&texture_root.join("misc").join("vignette.png"));
                assets.block_textures = load_block_textures(&texture_root.join("blocks"));
            }
            Err(error) => {
                notice = Some(format!("Vanilla textures unavailable: {error}"));
            }
        }

        (assets, notice)
    }

    pub fn block_texture(&self, block_state_id: u16, face: ChunkFace) -> Option<&ImageAsset> {
        let texture_name = block_texture_name(block_state_id, face);
        self.block_textures
            .get(texture_name)
            .or_else(|| self.block_textures.get("stone"))
            .or_else(|| self.block_textures.values().next())
    }
}

fn normalize_language_numeric_formats(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = String::new();
    let mut copied = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'%' {
            cursor += 1;
            continue;
        }
        let start = cursor;
        let digits_start = start + 1;
        let mut end = digits_start;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        let indexed = end > digits_start && bytes.get(end) == Some(&b'$');
        let index_end = if indexed { end + 1 } else { digits_start };
        end = index_end;
        while bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'.')
        {
            end += 1;
        }
        if matches!(bytes.get(end), Some(b'd' | b'f')) {
            output.push_str(&value[copied..start]);
            output.push('%');
            if indexed {
                output.push_str(&value[digits_start..index_end]);
            }
            output.push('s');
            cursor = end + 1;
            copied = cursor;
        } else {
            cursor = start + 1;
        }
    }
    output.push_str(&value[copied..]);
    output
}

pub(crate) fn parse_language_table(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), normalize_language_numeric_formats(value)))
        .collect()
}

fn load_block_textures(root: &Path) -> BTreeMap<String, ImageAsset> {
    let mut required = common_block_texture_names();
    required.insert("stone".to_owned());
    let mut textures = BTreeMap::new();

    for name in required {
        let path = root.join(format!("{name}.png"));
        if let Some(texture) = ImageAsset::load(&path) {
            textures.insert(name, texture);
        }
    }

    textures
}

fn common_block_texture_names() -> BTreeSet<String> {
    [
        "bedrock",
        "bookshelf",
        "brick",
        "clay",
        "coal_block",
        "coal_ore",
        "cobblestone",
        "crafting_table_front",
        "crafting_table_side",
        "crafting_table_top",
        "diamond_block",
        "diamond_ore",
        "dirt",
        "emerald_block",
        "emerald_ore",
        "end_stone",
        "glass",
        "gold_block",
        "gold_ore",
        "grass_side",
        "grass_top",
        "gravel",
        "hardened_clay",
        "iron_block",
        "iron_ore",
        "lapis_block",
        "lapis_ore",
        "leaves_acacia",
        "leaves_birch",
        "leaves_jungle",
        "leaves_oak",
        "leaves_spruce",
        "log_acacia",
        "log_acacia_top",
        "log_big_oak",
        "log_big_oak_top",
        "log_birch",
        "log_birch_top",
        "log_jungle",
        "log_jungle_top",
        "log_oak",
        "log_oak_top",
        "log_spruce",
        "log_spruce_top",
        "mossy_cobblestone",
        "nether_brick",
        "netherrack",
        "obsidian",
        "planks_acacia",
        "planks_big_oak",
        "planks_birch",
        "planks_jungle",
        "planks_oak",
        "planks_spruce",
        "quartz_block_bottom",
        "quartz_block_lines",
        "quartz_block_side",
        "quartz_block_top",
        "red_sand",
        "red_sandstone_bottom",
        "red_sandstone_carved",
        "red_sandstone_normal",
        "red_sandstone_smooth",
        "red_sandstone_top",
        "redstone_block",
        "sand",
        "sandstone_bottom",
        "sandstone_carved",
        "sandstone_normal",
        "sandstone_smooth",
        "sandstone_top",
        "snow",
        "soul_sand",
        "sponge",
        "stone",
        "stonebrick",
        "stonebrick_carved",
        "stonebrick_cracked",
        "stonebrick_mossy",
        "tnt_bottom",
        "tnt_side",
        "tnt_top",
        "water_still",
        "wool_colored_black",
        "wool_colored_blue",
        "wool_colored_brown",
        "wool_colored_cyan",
        "wool_colored_gray",
        "wool_colored_green",
        "wool_colored_light_blue",
        "wool_colored_lime",
        "wool_colored_magenta",
        "wool_colored_orange",
        "wool_colored_pink",
        "wool_colored_purple",
        "wool_colored_red",
        "wool_colored_silver",
        "wool_colored_white",
        "wool_colored_yellow",
        "hardened_clay_stained_black",
        "hardened_clay_stained_blue",
        "hardened_clay_stained_brown",
        "hardened_clay_stained_cyan",
        "hardened_clay_stained_gray",
        "hardened_clay_stained_green",
        "hardened_clay_stained_light_blue",
        "hardened_clay_stained_lime",
        "hardened_clay_stained_magenta",
        "hardened_clay_stained_orange",
        "hardened_clay_stained_pink",
        "hardened_clay_stained_purple",
        "hardened_clay_stained_red",
        "hardened_clay_stained_silver",
        "hardened_clay_stained_white",
        "hardened_clay_stained_yellow",
        "glass_black",
        "glass_blue",
        "glass_brown",
        "glass_cyan",
        "glass_gray",
        "glass_green",
        "glass_light_blue",
        "glass_lime",
        "glass_magenta",
        "glass_orange",
        "glass_pink",
        "glass_purple",
        "glass_red",
        "glass_silver",
        "glass_white",
        "glass_yellow",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn block_texture_name(block_state_id: u16, face: ChunkFace) -> &'static str {
    let block_id = block_state_id >> 4;
    let meta = (block_state_id & 0x000f) as u8;

    match block_id {
        1 => "stone",
        2 => match face {
            ChunkFace::Up => "grass_top",
            ChunkFace::Down => "dirt",
            _ => "grass_side",
        },
        3 => "dirt",
        4 => "cobblestone",
        5 => wood_planks_texture(meta),
        7 => "bedrock",
        8 | 9 => "water_still",
        12 => {
            if meta == 1 {
                "red_sand"
            } else {
                "sand"
            }
        }
        13 => "gravel",
        14 => "gold_ore",
        15 => "iron_ore",
        16 => "coal_ore",
        17 => log_texture(meta, face),
        18 => leaves_texture(meta),
        19 => "sponge",
        20 => "glass",
        21 => "lapis_ore",
        22 => "lapis_block",
        24 => sandstone_texture(meta, face),
        35 => wool_texture(meta),
        41 => "gold_block",
        42 => "iron_block",
        45 => "brick",
        46 => match face {
            ChunkFace::Up => "tnt_top",
            ChunkFace::Down => "tnt_bottom",
            _ => "tnt_side",
        },
        47 => match face {
            ChunkFace::Up | ChunkFace::Down => "planks_oak",
            ChunkFace::North | ChunkFace::South => "bookshelf",
            ChunkFace::West | ChunkFace::East => "bookshelf",
        },
        48 => "mossy_cobblestone",
        49 => "obsidian",
        56 => "diamond_ore",
        57 => "diamond_block",
        80 => "snow",
        82 => "clay",
        87 => "netherrack",
        88 => "soul_sand",
        95 => stained_glass_texture(meta),
        98 => stone_brick_texture(meta),
        121 => "end_stone",
        129 => "emerald_ore",
        133 => "emerald_block",
        152 => "redstone_block",
        155 => quartz_texture(meta, face),
        159 => stained_hardened_clay_texture(meta),
        172 => "hardened_clay",
        173 => "coal_block",
        _ => "stone",
    }
}

fn wood_planks_texture(meta: u8) -> &'static str {
    match meta & 0x7 {
        0 => "planks_oak",
        1 => "planks_spruce",
        2 => "planks_birch",
        3 => "planks_jungle",
        4 => "planks_acacia",
        5 => "planks_big_oak",
        _ => "planks_oak",
    }
}

fn log_texture(meta: u8, face: ChunkFace) -> &'static str {
    let top = matches!(face, ChunkFace::Up | ChunkFace::Down);
    match meta & 0x3 {
        0 => {
            if top {
                "log_oak_top"
            } else {
                "log_oak"
            }
        }
        1 => {
            if top {
                "log_spruce_top"
            } else {
                "log_spruce"
            }
        }
        2 => {
            if top {
                "log_birch_top"
            } else {
                "log_birch"
            }
        }
        3 => {
            if top {
                "log_jungle_top"
            } else {
                "log_jungle"
            }
        }
        _ => "log_oak",
    }
}

fn leaves_texture(meta: u8) -> &'static str {
    match meta & 0x3 {
        0 => "leaves_oak",
        1 => "leaves_spruce",
        2 => "leaves_birch",
        3 => "leaves_jungle",
        _ => "leaves_oak",
    }
}

fn sandstone_texture(meta: u8, face: ChunkFace) -> &'static str {
    match face {
        ChunkFace::Up => "sandstone_top",
        ChunkFace::Down => "sandstone_bottom",
        _ => match meta {
            1 => "sandstone_carved",
            2 => "sandstone_smooth",
            _ => "sandstone_normal",
        },
    }
}

fn wool_texture(meta: u8) -> &'static str {
    match meta {
        0 => "wool_colored_white",
        1 => "wool_colored_orange",
        2 => "wool_colored_magenta",
        3 => "wool_colored_light_blue",
        4 => "wool_colored_yellow",
        5 => "wool_colored_lime",
        6 => "wool_colored_pink",
        7 => "wool_colored_gray",
        8 => "wool_colored_silver",
        9 => "wool_colored_cyan",
        10 => "wool_colored_purple",
        11 => "wool_colored_blue",
        12 => "wool_colored_brown",
        13 => "wool_colored_green",
        14 => "wool_colored_red",
        15 => "wool_colored_black",
        _ => "wool_colored_white",
    }
}

fn stained_glass_texture(meta: u8) -> &'static str {
    match meta {
        0 => "glass_white",
        1 => "glass_orange",
        2 => "glass_magenta",
        3 => "glass_light_blue",
        4 => "glass_yellow",
        5 => "glass_lime",
        6 => "glass_pink",
        7 => "glass_gray",
        8 => "glass_silver",
        9 => "glass_cyan",
        10 => "glass_purple",
        11 => "glass_blue",
        12 => "glass_brown",
        13 => "glass_green",
        14 => "glass_red",
        15 => "glass_black",
        _ => "glass_white",
    }
}

fn stained_hardened_clay_texture(meta: u8) -> &'static str {
    match meta {
        0 => "hardened_clay_stained_white",
        1 => "hardened_clay_stained_orange",
        2 => "hardened_clay_stained_magenta",
        3 => "hardened_clay_stained_light_blue",
        4 => "hardened_clay_stained_yellow",
        5 => "hardened_clay_stained_lime",
        6 => "hardened_clay_stained_pink",
        7 => "hardened_clay_stained_gray",
        8 => "hardened_clay_stained_silver",
        9 => "hardened_clay_stained_cyan",
        10 => "hardened_clay_stained_purple",
        11 => "hardened_clay_stained_blue",
        12 => "hardened_clay_stained_brown",
        13 => "hardened_clay_stained_green",
        14 => "hardened_clay_stained_red",
        15 => "hardened_clay_stained_black",
        _ => "hardened_clay_stained_white",
    }
}

fn stone_brick_texture(meta: u8) -> &'static str {
    match meta {
        1 => "stonebrick_mossy",
        2 => "stonebrick_cracked",
        3 => "stonebrick_carved",
        _ => "stonebrick",
    }
}

fn quartz_texture(meta: u8, face: ChunkFace) -> &'static str {
    match meta {
        2 => {
            if matches!(face, ChunkFace::Up | ChunkFace::Down) {
                "quartz_block_top"
            } else {
                "quartz_block_lines"
            }
        }
        _ => match face {
            ChunkFace::Up => "quartz_block_top",
            ChunkFace::Down => "quartz_block_bottom",
            _ => "quartz_block_side",
        },
    }
}

fn ensure_vanilla_assets() -> Result<(PathBuf, bool), String> {
    let output_root = env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?
        .join("local_assets")
        .join(format!("vanilla-{VANILLA_VERSION}"));

    let marker = output_root
        .join("assets")
        .join("minecraft")
        .join("textures")
        .join("gui")
        .join("widgets.png");
    if marker.exists()
        && output_root
            .join("assets/minecraft/font/glyph_sizes.bin")
            .exists()
        && output_root
            .join("assets/minecraft/textures/font/ascii.png")
            .exists()
        && output_root
            .join("assets/minecraft/lang/en_US.lang")
            .exists()
        && output_root
            .join("assets/minecraft/textures/misc/vignette.png")
            .exists()
    {
        return Ok((output_root, false));
    }

    let jar_path = candidate_minecraft_jar_paths()
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            "Minecraft 1.8.9 jar not found. Put MCP-919 or the vanilla 1.8.9 version jar next to the repo."
                .to_owned()
        })?;

    extract_assets_from_jar(&jar_path, &output_root).map_err(|error| {
        format!(
            "failed to extract vanilla textures from {}: {error}",
            jar_path.display()
        )
    })?;

    Ok((output_root, true))
}

fn candidate_minecraft_jar_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = env::current_dir() {
        candidates.push(
            cwd.join("MCP-919")
                .join("jars")
                .join("versions")
                .join(VANILLA_VERSION)
                .join(format!("{VANILLA_VERSION}.jar")),
        );
    }

    if let Ok(appdata) = env::var("APPDATA") {
        candidates.push(
            PathBuf::from(appdata)
                .join(".minecraft")
                .join("versions")
                .join(VANILLA_VERSION)
                .join(format!("{VANILLA_VERSION}.jar")),
        );
    }

    candidates
}

fn extract_assets_from_jar(jar_path: &Path, output_root: &Path) -> io::Result<()> {
    let file = File::open(jar_path)?;
    let mut archive = ZipArchive::new(file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");

        if !should_extract_asset(&name) {
            continue;
        }

        let Some(relative) = name.strip_prefix("assets/") else {
            continue;
        };
        let destination = output_root.join("assets").join(relative);

        if entry.is_dir() {
            fs::create_dir_all(&destination)?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = File::create(destination)?;
        io::copy(&mut entry, &mut output)?;
    }

    Ok(())
}

fn should_extract_asset(name: &str) -> bool {
    if name.split('/').any(|part| part == ".." || part == ".")
        || name.contains('\\')
        || name.contains(':')
    {
        return false;
    }
    name == "assets/minecraft/lang/en_US.lang"
        || name == "assets/minecraft/font/glyph_sizes.bin"
        || name
            .strip_prefix("assets/minecraft/textures/font/unicode_page_")
            .and_then(|name| name.strip_suffix(".png"))
            .is_some_and(|name| {
                name.len() == 2 && name.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        || name == "assets/minecraft/textures/font/ascii.png"
        || name.starts_with("assets/minecraft/textures/blocks/")
        || name == "assets/minecraft/textures/gui/widgets.png"
        || name == "assets/minecraft/textures/gui/icons.png"
        || name == "assets/minecraft/textures/misc/vignette.png"
}

#[cfg(test)]
mod tests {
    use super::should_extract_asset;
    #[test]
    fn language_numeric_formats_follow_mcp_replacement_pattern() {
        let cases = [
            ("%d / %.2f / %3$08.2f", "%s / %s / %3$s"),
            ("%%d / %01$d / %..f", "%%s / %01$s / %s"),
            ("%s %D %+d %1$-2f", "%s %D %+d %1$-2f"),
            ("%0$d %2147483648$f", "%0$s %2147483648$s"),
            ("日本語 %2$.3f", "日本語 %2$s"),
        ];
        for (input, expected) in cases {
            let table = super::parse_language_table(&format!("key={input}"));
            assert_eq!(table["key"], expected);
        }
    }

    #[test]
    fn imports_only_supported_local_language_file() {
        assert!(should_extract_asset("assets/minecraft/lang/en_US.lang"));
        assert!(should_extract_asset(
            "assets/minecraft/font/glyph_sizes.bin"
        ));
        assert!(should_extract_asset(
            "assets/minecraft/textures/font/unicode_page_65.png"
        ));
        assert!(!should_extract_asset(
            "assets/minecraft/textures/font/unicode_page_../secret.png"
        ));
        assert!(!should_extract_asset(
            "assets/minecraft/textures/font/unicode_page_zz.png"
        ));
        assert!(!should_extract_asset("assets/minecraft/lang/../../secret"));
        assert!(!should_extract_asset("assets/minecraft/lang/ja_JP.lang"));
        let table = super::parse_language_table("#ignore\r\nkey=a=b\r\nempty=\r\n");
        assert_eq!(table["key"], "a=b");
        assert_eq!(table["empty"], "");
    }

    #[test]
    fn linear_sampling_uses_texel_centers_and_repeat_edges() {
        let image = super::ImageAsset {
            width: 2,
            height: 1,
            pixels: vec![0, 0, 0, 255, 255, 255, 255, 255],
        };
        assert_eq!(image.sample_linear_repeat(0.25, 0.5), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(image.sample_linear_repeat(0.75, 0.5), [1.0; 4]);
        assert_eq!(image.sample_linear_repeat(0.5, 0.5), [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(image.sample_linear_repeat(0.0, 0.5), [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(image.sample_linear_repeat(1.0, 0.5), [0.5, 0.5, 0.5, 1.0]);
    }
    #[test]
    fn local_jar_import_only_accepts_safe_asset_paths() {
        assert!(should_extract_asset(
            "assets/minecraft/textures/blocks/stone.png"
        ));
        assert!(!should_extract_asset(
            "assets/minecraft/textures/blocks/../../../../account.json"
        ));
        assert!(!should_extract_asset(
            "net/minecraft/client/Minecraft.class"
        ));
        assert!(!should_extract_asset(
            "assets/minecraft/sounds/random/click.ogg"
        ));
    }
}
