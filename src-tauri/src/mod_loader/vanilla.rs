use base64::Engine;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::model_resolver::{self, ModelJson, ModelRegistry, MultiNamespaceModels};
use super::texture_renderer;
use super::types::*;

/// Result of loading the vanilla JAR: (mod info, items, model registry, raw textures)
type VanillaLoadResult = (
    ModInfo,
    Vec<ItemInfo>,
    MultiNamespaceModels,
    HashMap<String, Vec<u8>>,
);

/// Load all vanilla items from the Minecraft client JAR.
///
/// The client JAR is a standard ZIP archive containing:
/// - `assets/minecraft/lang/en_us.json` — display names
/// - `assets/minecraft/textures/item/*.png` — item textures
/// - `assets/minecraft/textures/block/*.png` — block textures
/// - `assets/minecraft/models/item/*.json` — item model definitions
/// - `assets/minecraft/models/block/*.json` — block model definitions
pub fn load_vanilla_jar(jar_path: &Path, mc_version: &str) -> Result<VanillaLoadResult, String> {
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

    // Collect all textures from the JAR (including subdirectories)
    let textures = collect_all_textures(&mut archive, mod_id);

    // Collect all model JSONs from the JAR
    let model_registry = collect_models(&mut archive, mod_id);

    // Build the multi-namespace model registry (vanilla only has "minecraft")
    let mut all_models = MultiNamespaceModels::new();
    all_models.insert(mod_id.to_string(), model_registry);

    let mut items: Vec<ItemInfo> = Vec::new();

    // Build items from lang entries, using model resolver for textures
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

        // Try model-based resolution first
        let texture_base64 = resolve_texture_for_item(name, kind, &all_models, mod_id, &textures)
            .or_else(|| {
                // Fallback to direct texture lookup (original behavior)
                let texture_key = if kind == "item" {
                    format!("item/{}", name)
                } else {
                    format!("block/{}", name)
                };
                textures.get(&texture_key).map(|bytes| {
                    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                    format!("data:image/png;base64,{}", encoded)
                })
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
            // Only use top-level item/ and block/ textures for discovery
            let parts: Vec<&str> = texture_path.splitn(2, '/').collect();
            if parts.len() != 2 {
                continue;
            }
            let prefix = parts[0];
            let name = parts[1];
            if prefix != "item" && prefix != "block" {
                continue;
            }
            // Skip subdirectory textures (e.g. "block/door/oak_door_top")
            if name.contains('/') {
                continue;
            }

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

    Ok((mod_info, items, all_models, textures))
}

/// Resolve a texture for an item using the model chain, then render it.
/// Returns a data URI string if successful.
fn resolve_texture_for_item(
    item_name: &str,
    kind: &str,
    all_models: &MultiNamespaceModels,
    namespace: &str,
    all_textures: &HashMap<String, Vec<u8>>,
) -> Option<String> {
    let resolved = model_resolver::resolve_item_texture(item_name, kind, all_models, namespace)?;

    let png_bytes = texture_renderer::render_texture(&resolved, all_textures, namespace)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    Some(format!("data:image/png;base64,{}", encoded))
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

/// Collect ALL PNG textures from the JAR's assets/<mod_id>/textures/ directory,
/// including subdirectories. Returns a map of relative paths to raw PNG bytes.
/// Keys are like "item/diamond", "block/stone", "block/door/oak_door_top", etc.
fn collect_all_textures(archive: &mut ZipArchive<File>, mod_id: &str) -> HashMap<String, Vec<u8>> {
    let textures_prefix = format!("assets/{}/textures/", mod_id);
    let mut textures = HashMap::new();

    let file_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for file_name in &file_names {
        if !file_name.ends_with(".png") {
            continue;
        }

        if let Some(rest) = file_name.strip_prefix(&textures_prefix) {
            let rel_path = rest.strip_suffix(".png").unwrap_or(rest).to_string();

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

/// Collect all model JSONs from the JAR's assets/<mod_id>/models/ directory.
/// Returns a ModelRegistry mapping model paths (e.g. "item/diamond", "block/stone")
/// to parsed ModelJson structures.
fn collect_models(archive: &mut ZipArchive<File>, mod_id: &str) -> ModelRegistry {
    let models_prefix = format!("assets/{}/models/", mod_id);
    let mut models = ModelRegistry::new();

    let file_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for file_name in &file_names {
        if !file_name.ends_with(".json") {
            continue;
        }

        if let Some(rest) = file_name.strip_prefix(&models_prefix) {
            let model_path = rest.strip_suffix(".json").unwrap_or(rest).to_string();

            if let Ok(mut entry) = archive.by_name(file_name) {
                let mut contents = String::new();
                if entry.read_to_string(&mut contents).is_ok() {
                    if let Ok(model) = serde_json::from_str::<ModelJson>(&contents) {
                        models.insert(model_path, model);
                    }
                }
            }
        }
    }

    models
}
