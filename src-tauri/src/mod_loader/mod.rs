pub mod fabric;
pub mod kubejs;
pub mod types;
pub mod vanilla;

use base64::Engine;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use types::*;

/// Load all mods from a mods directory and optionally merge with KubeJS data.
///
/// Pipeline:
/// 0. If vanilla_jar is provided, parse the Minecraft client JAR for vanilla items
/// 1. Scan mods_dir for .jar files, parse each as a Fabric mod
/// 2. If kubejs_dir is provided, parse startup scripts for registered items
/// 3. Apply KubeJS texture and lang overrides on top of JAR data
/// 4. Return merged LoadedMods (Minecraft group pinned first)
///
/// `vanilla_jar` is `Some((jar_path, mc_version_string))`.
pub fn load_all(
    mods_dir: &Path,
    kubejs_dir: Option<&Path>,
    vanilla_jar: Option<(&Path, &str)>,
) -> LoadedMods {
    let mut all_mods: Vec<ModInfo> = Vec::new();
    let mut all_items: HashMap<String, Vec<ItemInfo>> = HashMap::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut total_items: usize = 0;

    // --- Phase 0: Load vanilla Minecraft items ---
    if let Some((jar_path, mc_version)) = vanilla_jar {
        match vanilla::load_vanilla_jar(jar_path, mc_version) {
            Ok((mod_info, items)) => {
                let mod_id = mod_info.id.clone();
                total_items += items.len();
                all_mods.push(mod_info);
                all_items
                    .entry(mod_id)
                    .or_insert_with(Vec::new)
                    .extend(items);
            }
            Err(e) => {
                warnings.push(format!("Vanilla Minecraft loading error: {}", e));
            }
        }
    }

    // --- Phase 1: Load Fabric mod JARs ---
    if mods_dir.exists() && mods_dir.is_dir() {
        let entries = match fs::read_dir(mods_dir) {
            Ok(e) => e,
            Err(e) => {
                warnings.push(format!("Failed to read mods directory: {}", e));
                return LoadedMods {
                    mods: all_mods,
                    items: all_items,
                    total_items,
                    warnings,
                };
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "jar" {
                continue;
            }

            match fabric::load_fabric_jar(&path) {
                Ok((mod_info, items)) => {
                    let mod_id = mod_info.id.clone();
                    total_items += items.len();
                    all_mods.push(mod_info);
                    all_items
                        .entry(mod_id)
                        .or_insert_with(Vec::new)
                        .extend(items);
                }
                Err(e) => {
                    warnings.push(format!(
                        "Skipping {}: {}",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown"),
                        e
                    ));
                }
            }
        }
    }

    // --- Phase 2: Load KubeJS data ---
    if let Some(kjs_dir) = kubejs_dir {
        if kjs_dir.exists() && kjs_dir.is_dir() {
            match kubejs::load_kubejs_dir(kjs_dir) {
                Ok((kjs_items, texture_overrides, lang_overrides)) => {
                    // Add KubeJS-registered items
                    if !kjs_items.is_empty() {
                        // Add "kubejs" as a mod source
                        let already_has_kubejs = all_mods.iter().any(|m| m.id == "kubejs");
                        if !already_has_kubejs {
                            all_mods.push(ModInfo {
                                id: "kubejs".to_string(),
                                name: "KubeJS".to_string(),
                                version: "custom".to_string(),
                                source: ModSource::KubeJs,
                            });
                        }

                        total_items += kjs_items.len();
                        all_items
                            .entry("kubejs".to_string())
                            .or_insert_with(Vec::new)
                            .extend(kjs_items);
                    }

                    // --- Phase 3: Apply overrides ---

                    // Apply texture overrides from kubejs/assets/
                    for (_mod_id, items) in all_items.iter_mut() {
                        for item in items.iter_mut() {
                            // Check for texture override: "namespace:item/name" or "namespace:block/name"
                            let override_keys = vec![
                                format!("{}:item/{}", item.mod_id, item.name),
                                format!("{}:block/{}", item.mod_id, item.name),
                            ];

                            for key in override_keys {
                                if let Some(bytes) = texture_overrides.get(&key) {
                                    let encoded =
                                        base64::engine::general_purpose::STANDARD.encode(bytes);
                                    item.texture_base64 =
                                        Some(format!("data:image/png;base64,{}", encoded));
                                    break;
                                }
                            }

                            // Apply lang overrides
                            let lang_keys = vec![
                                format!("item.{}.{}", item.mod_id, item.name),
                                format!("block.{}.{}", item.mod_id, item.name),
                            ];

                            for key in lang_keys {
                                if let Some(name) = lang_overrides.get(&key) {
                                    item.display_name = name.clone();
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warnings.push(format!("KubeJS loading error: {}", e));
                }
            }
        }
    }

    // Sort mods alphabetically, but always pin "minecraft" first
    all_mods.sort_by(|a, b| match (a.id.as_str(), b.id.as_str()) {
        ("minecraft", _) => std::cmp::Ordering::Less,
        (_, "minecraft") => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    // Sort items within each mod alphabetically
    for items in all_items.values_mut() {
        items.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        });
    }

    LoadedMods {
        mods: all_mods,
        items: all_items,
        total_items,
        warnings,
    }
}
