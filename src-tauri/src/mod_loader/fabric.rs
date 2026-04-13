use base64::Engine;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::types::*;

/// Load all items from a single Fabric mod JAR file.
///
/// Parses fabric.mod.json for mod metadata, then extracts:
/// - Item names from lang file (assets/<mod_id>/lang/en_us.json)
/// - Item textures from assets/<mod_id>/textures/item/*.png
/// - Block textures from assets/<mod_id>/textures/block/*.png
pub fn load_fabric_jar(jar_path: &Path) -> Result<(ModInfo, Vec<ItemInfo>), String> {
    let file = File::open(jar_path)
        .map_err(|e| format!("Failed to open {}: {}", jar_path.display(), e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read JAR {}: {}", jar_path.display(), e))?;

    // 1. Parse fabric.mod.json
    let mod_json = read_fabric_mod_json(&mut archive)?;
    let mod_id = mod_json.id.clone();

    let mod_info = ModInfo {
        id: mod_id.clone(),
        name: mod_json.name.unwrap_or_else(|| mod_id.clone()),
        version: mod_json.version.unwrap_or_else(|| "unknown".to_string()),
        source: ModSource::FabricJar,
    };

    // 2. Load lang file for display names
    let lang_map = read_lang_file(&mut archive, &mod_id);

    // 3. Collect all textures available in the JAR
    let textures = collect_textures(&mut archive, &mod_id);

    // 4. Build items list from lang entries + available textures
    let mut items = Vec::new();

    // Items from lang file entries
    for (lang_key, display_name) in &lang_map {
        // Match patterns like "item.<mod_id>.<name>" and "block.<mod_id>.<name>"
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
            mod_id: mod_id.clone(),
            name: name.to_string(),
            display_name: display_name.clone(),
            texture_base64,
            source: ModSource::FabricJar,
        });
    }

    // 5. If no lang entries found, fall back to discovering items by texture files
    if items.is_empty() {
        for (texture_path, bytes) in &textures {
            // texture_path is like "item/ruby" or "block/copper_ore"
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
                mod_id: mod_id.clone(),
                name: name.to_string(),
                display_name,
                texture_base64,
                source: ModSource::FabricJar,
            });
        }
    }

    Ok((mod_info, items))
}

/// Read and parse fabric.mod.json from a JAR archive
fn read_fabric_mod_json(archive: &mut ZipArchive<File>) -> Result<FabricModJson, String> {
    let mut entry = archive
        .by_name("fabric.mod.json")
        .map_err(|_| "No fabric.mod.json found in JAR (not a Fabric mod?)".to_string())?;

    let mut contents = String::new();
    entry
        .read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read fabric.mod.json: {}", e))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse fabric.mod.json: {}", e))
}

/// Read the en_us.json lang file from a JAR, returning a map of lang keys to display names
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

/// Collect all PNG textures from a JAR's assets/<mod_id>/textures/ directory.
/// Returns a map of relative paths (e.g. "item/ruby") to raw PNG bytes.
fn collect_textures(archive: &mut ZipArchive<File>, mod_id: &str) -> HashMap<String, Vec<u8>> {
    let item_prefix = format!("assets/{}/textures/item/", mod_id);
    let block_prefix = format!("assets/{}/textures/block/", mod_id);

    let mut textures = HashMap::new();

    // We need to collect file names first since we can't borrow archive mutably twice
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
