import type { ItemInfo } from "./item";

/** Mirrors the Rust ExportRecipeRequest */
export interface ExportRecipeRequest {
  grid_type: string;
  shapeless: boolean;
  inputs: RecipeSlot[];
  outputs: RecipeSlot[];
  count: number;
}

/** A slot in the recipe export */
export interface RecipeSlot {
  slot_id: string;
  item_id: string;
  row: number;
  col: number;
}

/** State of a single grid slot in the UI */
export interface GridSlotState {
  slotId: string;
  /** A fully-loaded item from the mod browser, or a placeholder if the item is unknown */
  item: ItemInfo | PlaceholderItem | null;
}

/**
 * Represents an item referenced in a recipe that isn't present in the
 * currently loaded mod browser. Carries only the item ID so the slot can
 * still render something meaningful.
 */
export interface PlaceholderItem {
  /** Discriminant so components can tell this apart from a full ItemInfo */
  _placeholder: true;
  id: string;
  display_name: string;
}

/** Type guard — true when `item` is a PlaceholderItem */
export function isPlaceholder(
  item: ItemInfo | PlaceholderItem | null,
): item is PlaceholderItem {
  return item != null && "_placeholder" in item && item._placeholder === true;
}

/** Mirrors the Rust ParsedRecipe */
export interface ParsedRecipe {
  recipe_type: "shaped" | "shapeless";
  inputs: RecipeSlot[];
  output_item_id: string;
  count: number;
}

/** Mirrors the Rust RecipeFileEntry */
export interface RecipeFileEntry {
  file_name: string;
  file_path: string;
  recipe: ParsedRecipe;
}

