use serde_json::{json, Value};
use std::collections::HashMap;

use super::ExportRecipeRequest;

/// Generate a vanilla Minecraft JSON recipe (1.19.2 format) from grid state.
///
/// Supports:
/// - Shaped crafting (minecraft:crafting_shaped) with automatic pattern optimization
/// - Shapeless crafting (minecraft:crafting_shapeless)
pub fn export_recipe(request: &ExportRecipeRequest) -> Result<Value, String> {
    if request.outputs.is_empty() {
        return Err("No output item specified".to_string());
    }
    if request.inputs.is_empty() {
        return Err("No input items specified".to_string());
    }

    let output_item = &request.outputs[0].item_id;
    let count = request.count.max(1);

    if request.shapeless {
        export_shapeless(output_item, count, &request.inputs)
    } else {
        export_shaped(output_item, count, &request.inputs)
    }
}

/// Generate a shaped crafting recipe (minecraft:crafting_shaped)
fn export_shaped(
    output_item: &str,
    count: u32,
    inputs: &[super::RecipeSlot],
) -> Result<Value, String> {
    // Build a grid representation
    // Find the bounding box of populated slots
    let min_row = inputs.iter().map(|s| s.row).min().unwrap_or(0);
    let max_row = inputs.iter().map(|s| s.row).max().unwrap_or(0);
    let min_col = inputs.iter().map(|s| s.col).min().unwrap_or(0);
    let max_col = inputs.iter().map(|s| s.col).max().unwrap_or(0);

    let rows = max_row - min_row + 1;
    let cols = max_col - min_col + 1;

    // Assign a unique key character to each unique item
    let mut item_to_key: HashMap<String, char> = HashMap::new();
    let key_chars: Vec<char> = "ABCDEFGHI".chars().collect();
    let mut next_key = 0;

    for input in inputs {
        if !item_to_key.contains_key(&input.item_id) {
            if next_key >= key_chars.len() {
                return Err("Too many unique items for shaped recipe (max 9)".to_string());
            }
            item_to_key.insert(input.item_id.clone(), key_chars[next_key]);
            next_key += 1;
        }
    }

    // Build pattern strings
    let mut pattern: Vec<String> = Vec::new();
    for row in 0..rows {
        let mut row_str = String::new();
        for col in 0..cols {
            let actual_row = row + min_row;
            let actual_col = col + min_col;

            let item_at_slot = inputs
                .iter()
                .find(|s| s.row == actual_row && s.col == actual_col);

            match item_at_slot {
                Some(slot) => {
                    row_str.push(*item_to_key.get(&slot.item_id).unwrap());
                }
                None => {
                    row_str.push(' ');
                }
            }
        }
        pattern.push(row_str);
    }

    // Build key map
    let mut key_map: HashMap<String, Value> = HashMap::new();
    for (item_id, key_char) in &item_to_key {
        key_map.insert(key_char.to_string(), json!({ "item": item_id }));
    }

    // Build result
    let mut result = json!({ "item": output_item });
    if count > 1 {
        result["count"] = json!(count);
    }

    Ok(json!({
        "type": "minecraft:crafting_shaped",
        "pattern": pattern,
        "key": key_map,
        "result": result
    }))
}

/// Generate a shapeless crafting recipe (minecraft:crafting_shapeless)
fn export_shapeless(
    output_item: &str,
    count: u32,
    inputs: &[super::RecipeSlot],
) -> Result<Value, String> {
    let ingredients: Vec<Value> = inputs
        .iter()
        .map(|slot| json!({ "item": slot.item_id }))
        .collect();

    let mut result = json!({ "item": output_item });
    if count > 1 {
        result["count"] = json!(count);
    }

    Ok(json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ingredients,
        "result": result
    }))
}
