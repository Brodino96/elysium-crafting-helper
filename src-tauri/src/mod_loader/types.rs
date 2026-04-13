use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata about a single mod (from fabric.mod.json or KubeJS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    /// Mod identifier (e.g. "techreborn", "kubejs")
    pub id: String,
    /// Human-readable mod name
    pub name: String,
    /// Mod version string
    pub version: String,
    /// Source of this mod's data
    pub source: ModSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModSource {
    /// Loaded from a .jar file in the mods folder
    FabricJar,
    /// Registered via KubeJS startup scripts
    KubeJs,
}

/// A single game item with its metadata and texture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInfo {
    /// Full namespaced ID (e.g. "techreborn:ruby")
    pub id: String,
    /// The mod/namespace this item belongs to (e.g. "techreborn")
    pub mod_id: String,
    /// Short item name without namespace (e.g. "ruby")
    pub name: String,
    /// Human-readable display name (e.g. "Ruby")
    pub display_name: String,
    /// Base64-encoded PNG texture data (prefixed with "data:image/png;base64,")
    /// None if no texture was found
    pub texture_base64: Option<String>,
    /// Where this item came from
    pub source: ModSource,
}

/// The complete result of loading all mods and items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedMods {
    /// All discovered mods
    pub mods: Vec<ModInfo>,
    /// All discovered items, grouped by mod_id
    pub items: HashMap<String, Vec<ItemInfo>>,
    /// Total number of items loaded
    pub total_items: usize,
    /// Any warnings/errors encountered during loading
    pub warnings: Vec<String>,
}

/// Represents the fabric.mod.json structure (only fields we care about)
#[derive(Debug, Deserialize)]
pub struct FabricModJson {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Texture override: maps "namespace:item/path" to the actual PNG bytes
pub type TextureOverrides = HashMap<String, Vec<u8>>;

/// Lang override: maps "item.namespace.name" to display name
pub type LangOverrides = HashMap<String, String>;
