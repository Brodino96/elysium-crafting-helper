use base64::Engine;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::model_resolver::{self, ModelJson, ModelRegistry, MultiNamespaceModels};
use super::texture_renderer;
use super::types::*;

/// Intermediate data extracted from a single Fabric JAR (I/O phase only).
/// This struct is produced by `parse_fabric_jar` which does all ZIP reading
/// without needing any shared state, making it safe to run in parallel.
pub struct ParsedFabricJar {
    pub mod_info: ModInfo,
    pub mod_id: String,
    pub lang_map: HashMap<String, String>,
    pub textures: HashMap<String, Vec<u8>>,
    pub model_registry: ModelRegistry,
}

/// Phase 1a: Parse a Fabric JAR — extract metadata, lang, textures, and models.
///
/// This is the I/O-heavy phase that reads and decompresses the ZIP archive.
/// It requires NO shared state and can be safely called from multiple threads
/// in parallel via rayon.
pub fn parse_fabric_jar(jar_path: &Path) -> Result<ParsedFabricJar, String> {
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

    // 3. Collect all textures available in the JAR (including subdirectories)
    let textures = collect_all_textures(&mut archive, &mod_id);

    // 4. Collect all model JSONs from the JAR
    let model_registry = collect_models(&mut archive, &mod_id);

    Ok(ParsedFabricJar {
        mod_info,
        mod_id,
        lang_map,
        textures,
        model_registry,
    })
}

/// Phase 1c: Resolve textures for a parsed Fabric mod's items using the
/// fully-populated shared model/texture registries.
///
/// This only *reads* the shared registries (immutable references), so it can
/// be safely called from multiple threads in parallel via rayon.
pub fn resolve_fabric_items(
    parsed: &ParsedFabricJar,
    shared_models: &MultiNamespaceModels,
    shared_textures: &HashMap<String, Vec<u8>>,
) -> Vec<ItemInfo> {
    let mod_id = &parsed.mod_id;
    let lang_map = &parsed.lang_map;
    let textures = &parsed.textures;

    let mut items = Vec::new();

    for (lang_key, display_name) in lang_map {
        let parts: Vec<&str> = lang_key.splitn(3, '.').collect();
        if parts.len() != 3 {
            continue;
        }
        let kind = parts[0]; // "item" or "block"
        let namespace = parts[1];
        let name = parts[2];

        if namespace != mod_id.as_str() {
            continue;
        }
        if kind != "item" && kind != "block" {
            continue;
        }

        // Try model-based resolution first
        let texture_base64 =
            resolve_texture_for_item(name, kind, shared_models, mod_id, textures, shared_textures)
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
            mod_id: mod_id.clone(),
            name: name.to_string(),
            display_name: display_name.clone(),
            texture_base64,
            source: ModSource::FabricJar,
        });
    }

    // If no lang entries found, fall back to discovering items by texture files
    if items.is_empty() {
        for (texture_path, bytes) in textures {
            let parts: Vec<&str> = texture_path.splitn(2, '/').collect();
            if parts.len() != 2 {
                continue;
            }
            let prefix = parts[0];
            let name = parts[1];
            if prefix != "item" && prefix != "block" {
                continue;
            }
            // Skip subdirectory textures for fallback discovery
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
                mod_id: mod_id.clone(),
                name: name.to_string(),
                display_name,
                texture_base64,
                source: ModSource::FabricJar,
            });
        }
    }

    items
}

/// Resolve a texture for an item using the model chain, then render it.
/// Returns a data URI string if successful.
/// `local_textures` are this mod's textures, `shared_textures` includes all namespaces.
fn resolve_texture_for_item(
    item_name: &str,
    kind: &str,
    all_models: &MultiNamespaceModels,
    namespace: &str,
    local_textures: &HashMap<String, Vec<u8>>,
    shared_textures: &HashMap<String, Vec<u8>>,
) -> Option<String> {
    let resolved = model_resolver::resolve_item_texture(item_name, kind, all_models, namespace)?;

    // Try local textures first, then fall back to shared (cross-namespace) textures
    let png_bytes = texture_renderer::render_texture(&resolved, local_textures, namespace)
        .or_else(|| texture_renderer::render_texture(&resolved, shared_textures, namespace))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    Some(format!("data:image/png;base64,{}", encoded))
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

/// Collect ALL PNG textures from a JAR's assets/<mod_id>/textures/ directory,
/// including subdirectories. Returns a map of relative paths to raw PNG bytes.
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

/// Collect all model JSONs from a JAR's assets/<mod_id>/models/ directory.
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
