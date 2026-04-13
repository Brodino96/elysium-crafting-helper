use base64::Engine;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use super::types::*;

/// Load items and asset overrides from a KubeJS directory.
///
/// Parses:
/// - startup_scripts/*.js for event.create() item registrations (KubeJS 6 / 1.19.2)
/// - assets/<namespace>/textures/ for texture overrides
/// - assets/<namespace>/lang/ for lang overrides
pub fn load_kubejs_dir(
    kubejs_path: &Path,
) -> Result<(Vec<ItemInfo>, TextureOverrides, LangOverrides), String> {
    let mut items = Vec::new();
    let mut texture_overrides = TextureOverrides::new();
    let mut lang_overrides = LangOverrides::new();

    // 1. Parse startup scripts for item registrations
    let startup_dir = kubejs_path.join("startup_scripts");
    if startup_dir.exists() && startup_dir.is_dir() {
        items = parse_startup_scripts(&startup_dir)?;
    }

    // 2. Load asset overrides (textures and lang files)
    let assets_dir = kubejs_path.join("assets");
    if assets_dir.exists() && assets_dir.is_dir() {
        load_asset_overrides(&assets_dir, &mut texture_overrides, &mut lang_overrides);
    }

    // 3. Resolve textures for KubeJS-registered items
    for item in &mut items {
        let texture_key = format!("{}:item/{}", item.mod_id, item.name);
        if let Some(bytes) = texture_overrides.get(&texture_key) {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            item.texture_base64 = Some(format!("data:image/png;base64,{}", encoded));
        }
    }

    Ok((items, texture_overrides, lang_overrides))
}

/// Parse all .js files in the startup_scripts directory for item registrations.
///
/// Handles KubeJS 6 (1.19.2) syntax:
///   StartupEvents.registry('item', event => { event.create('item_name') })
///
/// Extracts:
/// - Item name from event.create('name') or event.create('name', 'type')
/// - Display name from .displayName('...') if chained
/// - Custom texture from .texture('namespace:item/path') if chained
fn parse_startup_scripts(startup_dir: &Path) -> Result<Vec<ItemInfo>, String> {
    let mut items = Vec::new();

    // Regex patterns for KubeJS 6 item registration
    // Match event.create('item_name') or event.create('item_name', 'type')
    let create_re =
        Regex::new(r#"event\s*\.\s*create\s*\(\s*['"]([^'"]+)['"]\s*(?:,\s*['"][^'"]*['"]\s*)?\)"#)
            .map_err(|e| format!("Regex error: {}", e))?;

    // Match .displayName('...') chained after create
    let display_name_re = Regex::new(r#"\.displayName\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
        .map_err(|e| format!("Regex error: {}", e))?;

    // Match .texture('namespace:item/path') chained after create
    let texture_re = Regex::new(r#"\.texture\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
        .map_err(|e| format!("Regex error: {}", e))?;

    for entry in WalkDir::new(startup_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "js" && ext != "ts" {
            continue;
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        // Check if this script contains an item registry block
        if !content.contains("registry")
            || (!content.contains("'item'") && !content.contains("\"item\""))
        {
            continue;
        }

        // Find all event.create() calls
        // We process each line to also capture chained methods on the same line
        for line in content.lines() {
            if let Some(cap) = create_re.captures(line) {
                let item_name = cap[1].to_string();

                // Check for chained .displayName()
                let display_name = display_name_re
                    .captures(line)
                    .map(|c| c[1].to_string())
                    .unwrap_or_else(|| snake_to_title(&item_name));

                // Check for chained .texture() to determine namespace
                let _custom_texture = texture_re.captures(line).map(|c| c[1].to_string());

                // Default namespace for KubeJS items is "kubejs"
                let mod_id = "kubejs".to_string();

                items.push(ItemInfo {
                    id: format!("{}:{}", mod_id, item_name),
                    mod_id: mod_id.clone(),
                    name: item_name,
                    display_name,
                    texture_base64: None, // Will be resolved later
                    source: ModSource::KubeJs,
                });
            }
        }
    }

    Ok(items)
}

/// Load texture and lang overrides from the kubejs/assets/ directory.
///
/// Structure:
///   assets/<namespace>/textures/item/<name>.png  -> texture override
///   assets/<namespace>/textures/block/<name>.png -> texture override
///   assets/<namespace>/lang/en_us.json           -> lang override
fn load_asset_overrides(
    assets_dir: &Path,
    texture_overrides: &mut TextureOverrides,
    lang_overrides: &mut LangOverrides,
) {
    // Iterate over namespace directories
    let entries = match fs::read_dir(assets_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let namespace_path = entry.path();
        if !namespace_path.is_dir() {
            continue;
        }
        let namespace = match namespace_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Load texture overrides
        let textures_dir = namespace_path.join("textures");
        if textures_dir.exists() {
            load_texture_overrides(&textures_dir, &namespace, texture_overrides);
        }

        // Load lang overrides
        let lang_file = namespace_path.join("lang").join("en_us.json");
        if lang_file.exists() {
            if let Ok(content) = fs::read_to_string(&lang_file) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                    lang_overrides.extend(map);
                }
            }
        }
    }
}

/// Recursively load PNG textures from a textures/ directory
fn load_texture_overrides(
    textures_dir: &Path,
    namespace: &str,
    texture_overrides: &mut TextureOverrides,
) {
    for sub in &["item", "block"] {
        let sub_dir = textures_dir.join(sub);
        if !sub_dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&sub_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("png") {
                continue;
            }

            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if let Ok(bytes) = fs::read(path) {
                let key = format!("{}:{}/{}", namespace, sub, name);
                texture_overrides.insert(key, bytes);
            }
        }
    }
}

/// Convert a snake_case name to Title Case for display
fn snake_to_title(name: &str) -> String {
    name.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
