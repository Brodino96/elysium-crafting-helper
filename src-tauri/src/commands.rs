use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::mod_loader;
use crate::mod_loader::types::LoadedMods;
use crate::recipe::{vanilla_json, ExportRecipeRequest};

/// Application state holding the currently loaded mods and folder paths
pub struct AppState {
    pub modpack_dir: Mutex<Option<PathBuf>>,
    pub loaded_mods: Mutex<Option<LoadedMods>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            modpack_dir: Mutex::new(None),
            loaded_mods: Mutex::new(None),
        }
    }
}

/// Set the modpack root directory. Expects it to contain a `mods/` subfolder
/// (and optionally a `kubejs/` subfolder).
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

    *state.modpack_dir.lock().unwrap() = Some(path);
    Ok(())
}

/// Load (or reload) all mods from the configured modpack directory.
/// Looks for `<modpack_dir>/mods` and `<modpack_dir>/kubejs`.
#[tauri::command]
pub fn load_mods(state: State<'_, AppState>) -> Result<LoadedMods, String> {
    let modpack_dir = state.modpack_dir.lock().unwrap();
    let modpack_path = modpack_dir
        .as_ref()
        .ok_or("No modpack directory selected")?;

    let mods_path = modpack_path.join("mods");
    let kubejs_path = modpack_path.join("kubejs");
    let kubejs_opt = if kubejs_path.is_dir() {
        Some(kubejs_path.as_path())
    } else {
        None
    };

    let loaded = mod_loader::load_all(&mods_path, kubejs_opt);

    // Cache the loaded data
    *state.loaded_mods.lock().unwrap() = Some(loaded.clone());

    Ok(loaded)
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
