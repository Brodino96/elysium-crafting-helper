use base64::Engine;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::types::*;

/// Load all vanilla items from the Minecraft client JAR.
///
/// The client JAR is a standard ZIP archive containing:
/// - `assets/minecraft/lang/en_us.json` — display names
/// - `assets/minecraft/textures/item/*.png` — item textures
/// - `assets/minecraft/textures/block/*.png` — block textures
pub fn load_vanilla_jar(
    jar_path: &Path,
    mc_version: &str,
) -> Result<(ModInfo, Vec<ItemInfo>), String> {
    let file = File::open(jar_path)
        .map_err(|e| format!("Failed to open Minecraft JAR {}: {}", jar_path.display(), e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read Minecraft JAR {}: {}", jar_path.display(), e))?;

    let mod_id = "minecraft";

    let mod_info = ModInfo {
        id: mod_id.to_string(),
        name: "Minecraft".to_string(),
        version: mc_version.to_string(),
        source: ModSource::VanillaJar,
    };

    // Load lang file for display names
    let lang_map = read_lang_file(&mut archive, mod_id);

    // Collect all textures from the JAR
    let textures = collect_textures(&mut archive, mod_id);

    let mut items: Vec<ItemInfo> = Vec::new();

    // Build items from lang entries (same pattern as fabric.rs)
    for (lang_key, display_name) in &lang_map {
        // Match "item.minecraft.<name>" and "block.minecraft.<name>"
        let parts: Vec<&str> = lang_key.splitn(3, '.').collect();
        if parts.len() != 3 {
            continue;
        }
        let kind = parts[0]; // "item" or "block"
        let namespace = parts[1];
        let name = parts[2];

        if namespace != mod_id {
            continue;
        }
        if kind != "item" && kind != "block" {
            continue;
        }

        let texture_key = if kind == "item" {
            format!("item/{}", name)
        } else {
            format!("block/{}", name)
        };

        let texture_base64 = textures.get(&texture_key).map(|bytes| {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!("data:image/png;base64,{}", encoded)
        });

        items.push(ItemInfo {
            id: format!("{}:{}", mod_id, name),
            mod_id: mod_id.to_string(),
            name: name.to_string(),
            display_name: display_name.clone(),
            texture_base64,
            source: ModSource::VanillaJar,
        });
    }

    // Fallback: if no lang entries, discover items from texture filenames
    if items.is_empty() {
        for (texture_path, bytes) in &textures {
            // texture_path is like "item/diamond" or "block/stone"
            let parts: Vec<&str> = texture_path.splitn(2, '/').collect();
            if parts.len() != 2 {
                continue;
            }
            let name = parts[1];

            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            let texture_base64 = Some(format!("data:image/png;base64,{}", encoded));

            // Generate display name from item name (snake_case -> Title Case)
            let display_name = name
                .split('_')
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            items.push(ItemInfo {
                id: format!("{}:{}", mod_id, name),
                mod_id: mod_id.to_string(),
                name: name.to_string(),
                display_name,
                texture_base64,
                source: ModSource::VanillaJar,
            });
        }
    }

    Ok((mod_info, items))
}

/// Read the en_us.json lang file from the JAR, returning a map of lang keys to display names
fn read_lang_file(archive: &mut ZipArchive<File>, mod_id: &str) -> HashMap<String, String> {
    let lang_path = format!("assets/{}/lang/en_us.json", mod_id);

    let mut entry = match archive.by_name(&lang_path) {
        Ok(e) => e,
        Err(_) => return HashMap::new(),
    };

    let mut contents = String::new();
    if entry.read_to_string(&mut contents).is_err() {
        return HashMap::new();
    }

    serde_json::from_str(&contents).unwrap_or_default()
}

/// Collect all PNG textures from the JAR's assets/<mod_id>/textures/ directory.
/// Returns a map of relative paths (e.g. "item/diamond") to raw PNG bytes.
fn collect_textures(archive: &mut ZipArchive<File>, mod_id: &str) -> HashMap<String, Vec<u8>> {
    let item_prefix = format!("assets/{}/textures/item/", mod_id);
    let block_prefix = format!("assets/{}/textures/block/", mod_id);

    let mut textures = HashMap::new();

    // Collect file names first to avoid borrow conflicts
    let file_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for file_name in &file_names {
        if !file_name.ends_with(".png") {
            continue;
        }

        let relative_path = if let Some(rest) = file_name.strip_prefix(&item_prefix) {
            Some(format!(
                "item/{}",
                rest.strip_suffix(".png").unwrap_or(rest)
            ))
        } else if let Some(rest) = file_name.strip_prefix(&block_prefix) {
            Some(format!(
                "block/{}",
                rest.strip_suffix(".png").unwrap_or(rest)
            ))
        } else {
            None
        };

        if let Some(rel_path) = relative_path {
            if let Ok(mut entry) = archive.by_name(file_name) {
                let mut bytes = Vec::new();
                if entry.read_to_end(&mut bytes).is_ok() {
                    textures.insert(rel_path, bytes);
                }
            }
        }
    }

    textures
}
