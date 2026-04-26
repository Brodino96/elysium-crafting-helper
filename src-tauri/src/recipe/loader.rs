use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

use super::RecipeSlot;

/// A parsed input slot from a loaded recipe file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRecipeSlot {
    pub slot_id: String,
    pub item_id: String,
    pub row: usize,
    pub col: usize,
}

/// A fully parsed crafting recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRecipe {
    /// "shaped" or "shapeless"
    pub recipe_type: String,
    pub inputs: Vec<RecipeSlot>,
    pub output_item_id: String,
    pub count: u32,
}

/// A recipe file entry returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeFileEntry {
    pub file_name: String,
    pub file_path: String,
    pub recipe: ParsedRecipe,
}

/// Raw shaped recipe JSON shape (for deserialization)
#[derive(Debug, Deserialize)]
struct RawShapedRecipe {
    pattern: Vec<String>,
    key: std::collections::HashMap<String, RawIngredient>,
    result: RawResult,
}

/// Raw shapeless recipe JSON shape
#[derive(Debug, Deserialize)]
struct RawShapelessRecipe {
    ingredients: Vec<RawIngredient>,
    result: RawResult,
}

/// An ingredient can be `{ "item": "..." }` or just a string
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawIngredient {
    Object { item: String },
    StringId(String),
}

impl RawIngredient {
    fn item_id(&self) -> &str {
        match self {
            RawIngredient::Object { item } => item,
            RawIngredient::StringId(s) => s,
        }
    }
}

/// Result can be `{ "item": "...", "count": N }` or `{ "id": "...", "count": N }` (1.20.5+)
#[derive(Debug, Deserialize)]
struct RawResult {
    #[serde(alias = "id")]
    item: Option<String>,
    count: Option<u32>,
}

/// Scan `dir` recursively for `.json` files, parse each as a vanilla Minecraft
/// crafting recipe (shaped or shapeless only), and return the successfully
/// parsed entries. Files that cannot be parsed or are of an unsupported type
/// are silently skipped.
pub fn load_recipes_from_dir(dir: &Path) -> Vec<RecipeFileEntry> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "json" {
            continue;
        }

        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let json: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let recipe_type_field = match json.get("type").and_then(|t| t.as_str()) {
            Some(t) => t.to_string(),
            None => continue,
        };

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.json")
            .to_string();
        let file_path = path.to_string_lossy().to_string();

        let parsed = match recipe_type_field.as_str() {
            "minecraft:crafting_shaped" => parse_shaped(&json),
            "minecraft:crafting_shapeless" => parse_shapeless(&json),
            _ => None,
        };

        if let Some(recipe) = parsed {
            entries.push(RecipeFileEntry {
                file_name,
                file_path,
                recipe,
            });
        }
    }

    // Sort by file name for deterministic ordering in the UI
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    entries
}

fn parse_shaped(json: &serde_json::Value) -> Option<ParsedRecipe> {
    let raw: RawShapedRecipe = serde_json::from_value(json.clone()).ok()?;

    let result_item = raw.result.item.as_deref()?.to_string();
    let count = raw.result.count.unwrap_or(1).max(1);

    let mut inputs: Vec<RecipeSlot> = Vec::new();

    for (row_idx, row_str) in raw.pattern.iter().enumerate() {
        for (col_idx, ch) in row_str.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let key = ch.to_string();
            let ingredient = raw.key.get(&key)?;
            let item_id = ingredient.item_id().to_string();

            inputs.push(RecipeSlot {
                slot_id: format!("input_{}_{}", row_idx, col_idx),
                item_id,
                row: row_idx,
                col: col_idx,
            });
        }
    }

    if inputs.is_empty() {
        return None;
    }

    Some(ParsedRecipe {
        recipe_type: "shaped".to_string(),
        inputs,
        output_item_id: result_item,
        count,
    })
}

fn parse_shapeless(json: &serde_json::Value) -> Option<ParsedRecipe> {
    let raw: RawShapelessRecipe = serde_json::from_value(json.clone()).ok()?;

    let result_item = raw.result.item.as_deref()?.to_string();
    let count = raw.result.count.unwrap_or(1).max(1);

    if raw.ingredients.is_empty() {
        return None;
    }

    // Lay ingredients out left-to-right, top-to-bottom in a 3x3 grid
    let inputs: Vec<RecipeSlot> = raw
        .ingredients
        .iter()
        .enumerate()
        .map(|(i, ingredient)| {
            let row = i / 3;
            let col = i % 3;
            RecipeSlot {
                slot_id: format!("input_{}_{}", row, col),
                item_id: ingredient.item_id().to_string(),
                row,
                col,
            }
        })
        .collect();

    Some(ParsedRecipe {
        recipe_type: "shapeless".to_string(),
        inputs,
        output_item_id: result_item,
        count,
    })
}
