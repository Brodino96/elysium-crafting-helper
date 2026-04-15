use serde::Deserialize;
use std::collections::HashMap;

/// Raw Minecraft model JSON structure.
/// Models reference a parent and define texture variables + element faces.
#[derive(Debug, Deserialize, Clone)]
pub struct ModelJson {
    /// Parent model path, e.g. "minecraft:item/generated" or "block/cube_all"
    pub parent: Option<String>,
    /// Texture variable map, e.g. {"layer0": "minecraft:item/diamond", "all": "minecraft:block/stone"}
    pub textures: Option<HashMap<String, String>>,
    /// Model elements (checked for existence to detect 3D models; not deserialized further)
    #[allow(dead_code)]
    pub elements: Option<Vec<serde_json::Value>>,
    /// Display settings (ignored but included so deserialization doesn't fail)
    #[allow(dead_code)]
    pub display: Option<serde_json::Value>,
    /// GUI light mode (ignored)
    #[allow(dead_code)]
    pub gui_light: Option<String>,
    /// Ambientocclusion (ignored)
    #[allow(dead_code)]
    pub ambientocclusion: Option<bool>,
    /// Override predicates (ignored)
    #[allow(dead_code)]
    pub overrides: Option<Vec<serde_json::Value>>,
}

/// The resolved texture information for an item.
/// After walking the model parent chain, we know what textures to use.
#[derive(Debug, Clone)]
pub enum ResolvedTexture {
    /// A flat sprite item (like tools, ingots, food) — use this single texture
    Sprite(String),
    /// A 3D block rendered as an isometric composite — top, side, and optionally distinct faces
    BlockIsometric {
        top: String,
        front: String,
        right: String,
    },
}

/// Stores all parsed model JSONs for a given namespace, keyed by model path
/// e.g. "item/diamond_sword" -> ModelJson, "block/oak_planks" -> ModelJson
pub type ModelRegistry = HashMap<String, ModelJson>;

/// A collection of model registries from multiple namespaces.
/// Key is namespace (e.g. "minecraft"), value is that namespace's model registry.
pub type MultiNamespaceModels = HashMap<String, ModelRegistry>;

/// Well-known Minecraft builtin parent models that indicate a flat sprite item.
/// These are hardcoded in the Minecraft client and don't exist as JSON files.
const SPRITE_PARENTS: &[&str] = &[
    "item/generated",
    "minecraft:item/generated",
    "builtin/generated",
    "minecraft:builtin/generated",
    "item/handheld",
    "minecraft:item/handheld",
    "item/handheld_rod",
    "minecraft:item/handheld_rod",
];

/// Well-known parent models that indicate a 3D block model.
const BLOCK_PARENTS: &[&str] = &[
    "block/cube_all",
    "minecraft:block/cube_all",
    "block/cube_column",
    "minecraft:block/cube_column",
    "block/cube_column_horizontal",
    "minecraft:block/cube_column_horizontal",
    "block/cube_bottom_top",
    "minecraft:block/cube_bottom_top",
    "block/cube",
    "minecraft:block/cube",
    "block/orientable",
    "minecraft:block/orientable",
    "block/orientable_vertical",
    "minecraft:block/orientable_vertical",
];

/// Normalize a model path by stripping the "minecraft:" prefix if present.
/// E.g. "minecraft:block/stone" -> "block/stone", "block/stone" -> "block/stone"
fn normalize_model_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("minecraft:") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

/// Normalize a texture reference by stripping namespace prefix.
/// E.g. "minecraft:block/stone" -> "block/stone"
fn normalize_texture_ref(tex: &str) -> (String, String) {
    if let Some(idx) = tex.find(':') {
        let namespace = tex[..idx].to_string();
        let path = tex[idx + 1..].to_string();
        (namespace, path)
    } else {
        ("minecraft".to_string(), tex.to_string())
    }
}

/// Resolve all texture variables by walking the parent chain.
/// Returns the fully resolved texture map (all variables expanded).
fn resolve_texture_vars(
    model_path: &str,
    models: &MultiNamespaceModels,
    namespace: &str,
    depth: u32,
) -> HashMap<String, String> {
    if depth > 20 {
        return HashMap::new(); // prevent infinite loops
    }

    let registry = match models.get(namespace) {
        Some(r) => r,
        None => return HashMap::new(),
    };

    let model = match registry.get(model_path) {
        Some(m) => m.clone(),
        None => return HashMap::new(),
    };

    // Start with parent's textures (if any)
    let mut textures = if let Some(ref parent) = model.parent {
        let norm_parent = normalize_model_path(parent);
        // Parent might be in a different namespace
        let (parent_ns, parent_path) = if parent.contains(':') {
            normalize_texture_ref(parent)
        } else {
            (namespace.to_string(), norm_parent)
        };
        resolve_texture_vars(&parent_path, models, &parent_ns, depth + 1)
    } else {
        HashMap::new()
    };

    // Overlay this model's textures on top
    if let Some(ref model_textures) = model.textures {
        for (key, value) in model_textures {
            textures.insert(key.clone(), value.clone());
        }
    }

    // Resolve variable references (textures that start with '#')
    let mut resolved = textures.clone();
    for (_key, value) in resolved.iter_mut() {
        if value.starts_with('#') {
            let var_name = &value[1..];
            if let Some(actual) = textures.get(var_name) {
                if !actual.starts_with('#') {
                    *value = actual.clone();
                }
            }
        }
    }

    resolved
}

/// Check if a model (or any of its parents) is a sprite-type model.
fn is_sprite_model(
    model_path: &str,
    models: &MultiNamespaceModels,
    namespace: &str,
    depth: u32,
) -> bool {
    if depth > 20 {
        return false;
    }

    let check_path = format!("{}:{}", namespace, model_path);
    if SPRITE_PARENTS.contains(&check_path.as_str()) || SPRITE_PARENTS.contains(&model_path) {
        return true;
    }

    let registry = match models.get(namespace) {
        Some(r) => r,
        None => return false,
    };

    let model = match registry.get(model_path) {
        Some(m) => m,
        None => return false,
    };

    if let Some(ref parent) = model.parent {
        let norm = normalize_model_path(parent);
        if SPRITE_PARENTS.contains(&parent.as_str()) || SPRITE_PARENTS.contains(&norm.as_str()) {
            return true;
        }
        let (parent_ns, parent_path) = if parent.contains(':') {
            normalize_texture_ref(parent)
        } else {
            (namespace.to_string(), norm)
        };
        return is_sprite_model(&parent_path, models, &parent_ns, depth + 1);
    }

    false
}

/// Detect the block parent type by walking the parent chain.
/// Returns the normalized terminal parent name if it's a known block parent.
fn detect_block_parent(
    model_path: &str,
    models: &MultiNamespaceModels,
    namespace: &str,
    depth: u32,
) -> Option<String> {
    if depth > 20 {
        return None;
    }

    let check_path = format!("{}:{}", namespace, model_path);
    if BLOCK_PARENTS.contains(&check_path.as_str()) || BLOCK_PARENTS.contains(&model_path) {
        return Some(normalize_model_path(model_path));
    }

    let registry = models.get(namespace)?;

    let model = registry.get(model_path)?;

    if let Some(ref parent) = model.parent {
        let norm = normalize_model_path(parent);
        if BLOCK_PARENTS.contains(&parent.as_str()) || BLOCK_PARENTS.contains(&norm.as_str()) {
            return Some(norm);
        }
        let (parent_ns, parent_path) = if parent.contains(':') {
            normalize_texture_ref(parent)
        } else {
            (namespace.to_string(), norm)
        };
        return detect_block_parent(&parent_path, models, &parent_ns, depth + 1);
    }

    None
}

/// Resolve the texture for an item given its item model path (e.g. "item/diamond_sword").
///
/// Strategy:
/// 1. Parse the item's model JSON
/// 2. Walk the parent chain to collect all texture variables
/// 3. If it's a sprite model (parent chain leads to item/generated), use `layer0`
/// 4. If it's a block model, figure out top/front/right faces for isometric rendering
/// 5. Fall back to any available texture variable
pub fn resolve_item_texture(
    item_name: &str,
    kind: &str,
    models: &MultiNamespaceModels,
    namespace: &str,
) -> Option<ResolvedTexture> {
    let item_model_path = format!("item/{}", item_name);

    // First try to resolve from the item model
    let resolved = try_resolve_from_model(&item_model_path, models, namespace);
    if resolved.is_some() {
        return resolved;
    }

    // For blocks, also try the block model directly
    if kind == "block" {
        let block_model_path = format!("block/{}", item_name);
        let resolved = try_resolve_from_model(&block_model_path, models, namespace);
        if resolved.is_some() {
            return resolved;
        }
    }

    None
}

/// Try to resolve a texture from a specific model path.
fn try_resolve_from_model(
    model_path: &str,
    models: &MultiNamespaceModels,
    namespace: &str,
) -> Option<ResolvedTexture> {
    let textures = resolve_texture_vars(model_path, models, namespace, 0);
    if textures.is_empty() {
        return None;
    }

    // Check if this is a sprite model
    if is_sprite_model(model_path, models, namespace, 0) {
        // Use layer0 for sprite items
        if let Some(tex) = textures.get("layer0") {
            return Some(ResolvedTexture::Sprite(tex.clone()));
        }
    }

    // Check if this is a known block model type
    if let Some(block_parent) = detect_block_parent(model_path, models, namespace, 0) {
        return resolve_block_textures(&block_parent, &textures);
    }

    // Fallback: check if the item model has a parent that is a block model (common pattern)
    // e.g. item/oak_planks has parent: "block/oak_planks"
    let registry = models.get(namespace)?;
    let model = registry.get(model_path)?;
    if let Some(ref parent) = model.parent {
        let norm_parent = normalize_model_path(parent);
        if norm_parent.starts_with("block/") {
            let (parent_ns, parent_path) = if parent.contains(':') {
                normalize_texture_ref(parent)
            } else {
                (namespace.to_string(), norm_parent)
            };

            // Re-resolve textures from the block model
            let block_textures = resolve_texture_vars(&parent_path, models, &parent_ns, 0);
            if !block_textures.is_empty() {
                if let Some(bp) = detect_block_parent(&parent_path, models, &parent_ns, 0) {
                    return resolve_block_textures(&bp, &block_textures);
                }
                // Even without a known block parent, try to extract face textures
                return try_extract_any_block_texture(&block_textures);
            }
        }
    }

    // Last resort: use any texture we found
    if let Some(tex) = textures.get("layer0") {
        return Some(ResolvedTexture::Sprite(tex.clone()));
    }
    // Try "particle" or "texture" as common fallbacks
    if let Some(tex) = textures.get("particle") {
        return Some(ResolvedTexture::Sprite(tex.clone()));
    }
    if let Some(tex) = textures.get("texture") {
        return Some(ResolvedTexture::Sprite(tex.clone()));
    }
    // Use the first texture we find
    if let Some((_key, tex)) = textures.iter().next() {
        if !tex.starts_with('#') {
            return Some(ResolvedTexture::Sprite(tex.clone()));
        }
    }

    None
}

/// Given a known block parent type and resolved texture variables,
/// determine the top/front/right textures for isometric rendering.
fn resolve_block_textures(
    block_parent: &str,
    textures: &HashMap<String, String>,
) -> Option<ResolvedTexture> {
    match block_parent {
        "block/cube_all" => {
            let tex = textures.get("all")?;
            Some(ResolvedTexture::BlockIsometric {
                top: tex.clone(),
                front: tex.clone(),
                right: tex.clone(),
            })
        }
        "block/cube_column" | "block/cube_column_horizontal" => {
            let side = textures.get("side").cloned();
            let end = textures.get("end").cloned();
            let s = side.as_deref().or(end.as_deref())?;
            let e = end.as_deref().or(side.as_deref())?;
            Some(ResolvedTexture::BlockIsometric {
                top: e.to_string(),
                front: s.to_string(),
                right: s.to_string(),
            })
        }
        "block/cube_bottom_top" => {
            let top = textures.get("top").cloned();
            let side = textures.get("side").cloned();
            let t = top.as_deref().or(side.as_deref())?;
            let s = side.as_deref().or(top.as_deref())?;
            Some(ResolvedTexture::BlockIsometric {
                top: t.to_string(),
                front: s.to_string(),
                right: s.to_string(),
            })
        }
        "block/cube" => {
            let up = textures.get("up").cloned();
            let south = textures.get("south").cloned();
            let east = textures.get("east").cloned();
            let north = textures.get("north").cloned();
            let west = textures.get("west").cloned();
            let particle = textures.get("particle").cloned();
            let t = up.as_deref().or(particle.as_deref()).unwrap_or_default();
            let f = south
                .as_deref()
                .or(north.as_deref())
                .or(particle.as_deref())
                .unwrap_or_default();
            let r = east
                .as_deref()
                .or(west.as_deref())
                .or(particle.as_deref())
                .unwrap_or_default();
            if t.is_empty() {
                return None;
            }
            Some(ResolvedTexture::BlockIsometric {
                top: t.to_string(),
                front: f.to_string(),
                right: r.to_string(),
            })
        }
        "block/orientable" | "block/orientable_vertical" => {
            let top = textures.get("top").cloned();
            let front = textures.get("front").cloned();
            let side = textures.get("side").cloned();
            let t = top.as_deref().or(side.as_deref()).or(front.as_deref())?;
            let f = front.as_deref().or(side.as_deref())?;
            let s = side.as_deref().or(front.as_deref())?;
            Some(ResolvedTexture::BlockIsometric {
                top: t.to_string(),
                front: f.to_string(),
                right: s.to_string(),
            })
        }
        _ => try_extract_any_block_texture(textures),
    }
}

/// Try to extract block textures from texture variables using common naming conventions.
fn try_extract_any_block_texture(textures: &HashMap<String, String>) -> Option<ResolvedTexture> {
    // Try common texture variable names for block faces
    let top_candidates = ["top", "up", "end", "all"];
    let side_candidates = ["side", "front", "south", "north", "east", "west", "all"];

    let mut top = None;
    let mut side = None;

    for key in &top_candidates {
        if let Some(tex) = textures.get(*key) {
            if !tex.starts_with('#') {
                top = Some(tex.clone());
                break;
            }
        }
    }

    for key in &side_candidates {
        if let Some(tex) = textures.get(*key) {
            if !tex.starts_with('#') {
                side = Some(tex.clone());
                break;
            }
        }
    }

    // If we found at least a side texture, use it
    let s = side.or_else(|| top.clone())?;
    let t = top.unwrap_or_else(|| s.clone());

    Some(ResolvedTexture::BlockIsometric {
        top: t,
        front: s.clone(),
        right: s,
    })
}
