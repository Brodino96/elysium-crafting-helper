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
  item: ItemInfo | null;
}
