use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::mod_loader;
use crate::mod_loader::types::LoadedMods;
use crate::recipe::{vanilla_json, ExportRecipeRequest};

/// Minimal representation of minecraftinstance.json (CurseForge format)
#[derive(serde::Deserialize)]
struct MinecraftInstanceJson {
    #[serde(rename = "baseModLoader")]
    base_mod_loader: Option<BaseModLoaderJson>,
}

#[derive(serde::Deserialize)]
struct BaseModLoaderJson {
    #[serde(rename = "minecraftVersion")]
    minecraft_version: Option<String>,
}

/// Application state holding the currently loaded mods and folder paths
pub struct AppState {
    pub modpack_dir: Mutex<Option<PathBuf>>,
    pub loaded_mods: Mutex<Option<LoadedMods>>,
    pub minecraft_version: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            modpack_dir: Mutex::new(None),
            loaded_mods: Mutex::new(None),
            minecraft_version: Mutex::new(None),
        }
    }
}

/// Set the modpack root directory. Expects it to contain a `mods/` subfolder
/// (and optionally a `kubejs/` subfolder).
/// Also attempts to read `minecraftinstance.json` to detect the Minecraft version.
#[tauri::command]
pub fn set_modpack_dir(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid directory: {}", path.display()));
    }

    let mods_sub = path.join("mods");
    if !mods_sub.exists() || !mods_sub.is_dir() {
        return Err(format!(
            "No 'mods' folder found in {}. Select the modpack root directory.",
            path.display()
        ));
    }

    // Attempt to detect the Minecraft version from minecraftinstance.json
    let mc_version = read_minecraft_version(&path);
    *state.minecraft_version.lock().unwrap() = mc_version;

    *state.modpack_dir.lock().unwrap() = Some(path);
    Ok(())
}

/// Try to read `baseModLoader.minecraftVersion` from `minecraftinstance.json`.
/// Returns None silently if the file is missing or malformed.
fn read_minecraft_version(modpack_dir: &PathBuf) -> Option<String> {
    let instance_path = modpack_dir.join("minecraftinstance.json");
    let contents = std::fs::read_to_string(&instance_path).ok()?;
    let instance: MinecraftInstanceJson = serde_json::from_str(&contents).ok()?;
    instance.base_mod_loader?.minecraft_version
}

/// Get the cache directory for a modpack.
/// Returns `<OS temp dir>/elysium-crafting-helper/<modpack_folder_name>/`.
fn get_cache_dir(modpack_path: &std::path::Path) -> PathBuf {
    let folder_name = modpack_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    std::env::temp_dir()
        .join("elysium-crafting-helper")
        .join(folder_name)
}

/// Try to load cached mod data from disk.
fn load_from_cache(cache_dir: &std::path::Path) -> Option<LoadedMods> {
    let cache_file = cache_dir.join("cache.json");
    let contents = std::fs::read_to_string(&cache_file).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Save mod data to the cache directory on disk.
fn save_to_cache(cache_dir: &std::path::Path, data: &LoadedMods) -> Result<(), String> {
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    let cache_file = cache_dir.join("cache.json");
    let json =
        serde_json::to_string(data).map_err(|e| format!("Failed to serialize cache: {}", e))?;

    std::fs::write(&cache_file, json).map_err(|e| format!("Failed to write cache file: {}", e))?;

    Ok(())
}

/// Check whether a cache exists for the current modpack.
#[tauri::command]
pub fn has_cache(state: State<'_, AppState>) -> Result<bool, String> {
    let modpack_dir = state.modpack_dir.lock().unwrap();
    let modpack_path = modpack_dir
        .as_ref()
        .ok_or("No modpack directory selected")?;

    let cache_dir = get_cache_dir(modpack_path);
    Ok(cache_dir.join("cache.json").exists())
}

/// Load (or reload) all mods from the configured modpack directory.
///
/// When `from_mods` is `false`, tries to load from the disk cache first.
/// When `from_mods` is `true`, always re-parses all JARs and regenerates the cache.
///
/// Looks for `<modpack_dir>/mods` and `<modpack_dir>/kubejs`.
/// If a Minecraft version was detected, also loads vanilla items from the
/// CurseForge install directory at `<modpack_dir>/../../Install/versions/<ver>/<ver>.jar`.
#[tauri::command]
pub fn load_mods(from_mods: bool, state: State<'_, AppState>) -> Result<LoadedMods, String> {
    let modpack_dir = state.modpack_dir.lock().unwrap();
    let modpack_path = modpack_dir
        .as_ref()
        .ok_or("No modpack directory selected")?;

    let cache_dir = get_cache_dir(modpack_path);

    // Try loading from cache if not forcing a reload
    if !from_mods {
        if let Some(cached) = load_from_cache(&cache_dir) {
            *state.loaded_mods.lock().unwrap() = Some(cached.clone());
            return Ok(cached);
        }
    }

    // Full reload from mod JARs
    let mods_path = modpack_path.join("mods");
    let kubejs_path = modpack_path.join("kubejs");
    let kubejs_opt = if kubejs_path.is_dir() {
        Some(kubejs_path.as_path())
    } else {
        None
    };

    // Resolve vanilla JAR path: <modpack_dir>/../../Install/versions/<ver>/<ver>.jar
    // Read MC version and build the owned (PathBuf, String) pair in one lock scope.
    let vanilla_opt_owned: Option<(PathBuf, String)> = {
        let mc_version_guard = state.minecraft_version.lock().unwrap();
        mc_version_guard.as_ref().map(|version| {
            let jar_path = modpack_path
                .join("../../Install/versions")
                .join(version)
                .join(format!("{}.jar", version));
            (normalize_path(&jar_path), version.clone())
        })
    };

    let mut warnings_extra: Vec<String> = Vec::new();

    let vanilla_arg: Option<(&std::path::Path, &str)> =
        if let Some((ref jar, ref ver)) = vanilla_opt_owned {
            if jar.exists() {
                Some((jar.as_path(), ver.as_str()))
            } else {
                warnings_extra.push(format!(
                    "Minecraft JAR not found at '{}', vanilla items skipped. \
                     Check that the CurseForge install directory is at the expected location.",
                    jar.display()
                ));
                None
            }
        } else {
            warnings_extra.push(
                "minecraftinstance.json not found or missing baseModLoader.minecraftVersion; \
                 vanilla items skipped."
                    .to_string(),
            );
            None
        };

    let mut loaded = mod_loader::load_all(&mods_path, kubejs_opt, vanilla_arg);
    loaded.warnings.extend(warnings_extra);

    // Save to disk cache (non-fatal if it fails)
    if let Err(e) = save_to_cache(&cache_dir, &loaded) {
        loaded.warnings.push(format!("Cache write warning: {}", e));
    }

    // Cache in memory
    *state.loaded_mods.lock().unwrap() = Some(loaded.clone());

    Ok(loaded)
}

/// Normalize a path by resolving `.` and `..` components without requiring the path to exist.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

/// Export a recipe as vanilla JSON
#[tauri::command]
pub fn export_recipe(request: ExportRecipeRequest) -> Result<String, String> {
    let recipe_json = vanilla_json::export_recipe(&request)?;
    serde_json::to_string_pretty(&recipe_json)
        .map_err(|e| format!("Failed to serialize recipe: {}", e))
}

/// Get the currently loaded mods (without re-scanning)
#[tauri::command]
pub fn get_loaded_mods(state: State<'_, AppState>) -> Result<LoadedMods, String> {
    let loaded = state.loaded_mods.lock().unwrap();
    loaded.clone().ok_or("No mods loaded yet".to_string())
}

/// Save a string to a file at the given path
#[tauri::command]
pub fn save_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file {}: {}", path, e))
}
