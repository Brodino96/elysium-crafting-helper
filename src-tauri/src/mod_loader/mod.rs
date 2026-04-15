pub mod fabric;
pub mod kubejs;
pub mod model_resolver;
pub mod texture_renderer;
pub mod types;
pub mod vanilla;

use base64::Engine;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use model_resolver::MultiNamespaceModels;
use types::*;

/// Load all mods from a mods directory and optionally merge with KubeJS data.
///
/// Pipeline:
/// 0. If vanilla_jar is provided, parse the Minecraft client JAR for vanilla items
///    (also extracts models + textures for cross-mod resolution)
/// 1a. Scan mods_dir for .jar files, parse each in **parallel** (rayon) —
///     extracts metadata, lang, textures, models with NO shared state
/// 1b. Merge all parsed data into the shared model/texture registries (sequential, fast)
/// 1c. Resolve textures for all mods in **parallel** (rayon) —
///     uses the now-complete shared registries as read-only references
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

    // Shared model and texture registries across all namespaces.
    // Vanilla models are loaded first so fabric mods can reference them as parents.
    let mut shared_models = MultiNamespaceModels::new();
    let mut shared_textures: HashMap<String, Vec<u8>> = HashMap::new();

    // --- Phase 0: Load vanilla Minecraft items ---
    if let Some((jar_path, mc_version)) = vanilla_jar {
        match vanilla::load_vanilla_jar(jar_path, mc_version) {
            Ok((mod_info, items, vanilla_models, vanilla_textures)) => {
                let mod_id = mod_info.id.clone();
                total_items += items.len();
                all_mods.push(mod_info);

                // Merge vanilla models into the shared registry
                for (ns, registry) in vanilla_models {
                    shared_models.entry(ns).or_default().extend(registry);
                }

                // Merge vanilla textures into shared textures with namespace prefix
                for (path, bytes) in vanilla_textures {
                    shared_textures.insert(format!("minecraft:{}", path), bytes);
                }

                all_items.entry(mod_id).or_default().extend(items);
            }
            Err(e) => {
                warnings.push(format!("Vanilla Minecraft loading error: {}", e));
            }
        }
    }

    // --- Phase 1: Load Fabric mod JARs (parallel) ---
    if mods_dir.exists() && mods_dir.is_dir() {
        let jar_paths: Vec<PathBuf> = match fs::read_dir(mods_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jar"))
                .collect(),
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

        // Phase 1a: Parse all JARs in parallel (I/O-heavy, no shared state)
        let parse_results: Vec<_> = jar_paths
            .par_iter()
            .map(|path| (path.clone(), fabric::parse_fabric_jar(path)))
            .collect();

        // Phase 1b: Merge parsed data into shared registries (sequential, fast)
        let mut parsed_jars: Vec<fabric::ParsedFabricJar> = Vec::new();

        for (path, result) in parse_results {
            match result {
                Ok(parsed) => {
                    // Add this mod's models to the shared registry
                    shared_models.insert(parsed.mod_id.clone(), parsed.model_registry.clone());

                    // Merge this mod's textures into shared textures with namespace prefix
                    for (tex_path, bytes) in &parsed.textures {
                        shared_textures
                            .insert(format!("{}:{}", parsed.mod_id, tex_path), bytes.clone());
                    }

                    parsed_jars.push(parsed);
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

        // Phase 1c: Resolve textures for all mods in parallel (read-only shared state)
        let resolved: Vec<(ModInfo, Vec<ItemInfo>)> = parsed_jars
            .par_iter()
            .map(|parsed| {
                let items = fabric::resolve_fabric_items(parsed, &shared_models, &shared_textures);
                (parsed.mod_info.clone(), items)
            })
            .collect();

        for (mod_info, items) in resolved {
            let mod_id = mod_info.id.clone();
            total_items += items.len();
            all_mods.push(mod_info);
            all_items.entry(mod_id).or_default().extend(items);
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
                            .or_default()
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
