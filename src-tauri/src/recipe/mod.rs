use serde::{Deserialize, Serialize};

pub mod loader;
pub mod vanilla_json;

pub use loader::{load_recipes_from_dir, ParsedRecipe, RecipeFileEntry};

/// A recipe slot as received from the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSlot {
    /// Slot ID (e.g. "input_0_0", "output_0")
    pub slot_id: String,
    /// Item ID placed in this slot (e.g. "minecraft:diamond")
    pub item_id: String,
    /// Row position in the grid
    pub row: usize,
    /// Column position in the grid
    pub col: usize,
}

/// Request to export a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecipeRequest {
    /// Type of crafting grid (e.g. "crafting_table")
    pub grid_type: String,
    /// Whether this is a shapeless recipe
    pub shapeless: bool,
    /// Input slots with items
    pub inputs: Vec<RecipeSlot>,
    /// Output slot(s) with result item
    pub outputs: Vec<RecipeSlot>,
    /// Result count
    pub count: u32,
}
