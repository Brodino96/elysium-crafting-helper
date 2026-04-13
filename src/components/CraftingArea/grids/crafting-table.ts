import type {
  CraftingGridConfig,
  SlotDefinition,
} from "./types";
import type { GridSlotState, ExportRecipeRequest, RecipeSlot } from "../../../types/recipe";

/**
 * Standard Minecraft 3x3 Crafting Table grid.
 *
 * Layout:
 *   [0,0] [0,1] [0,2]
 *   [1,0] [1,1] [1,2]   →  [output]
 *   [2,0] [2,1] [2,2]
 */

function buildSlots(): SlotDefinition[] {
  const slots: SlotDefinition[] = [];

  // 3x3 input grid
  for (let row = 0; row < 3; row++) {
    for (let col = 0; col < 3; col++) {
      slots.push({
        id: `input_${row}_${col}`,
        type: "input",
        row,
        col,
      });
    }
  }

  // Single output slot
  slots.push({
    id: "output_0",
    type: "output",
    row: 1, // vertically centered
    col: 4, // after the 3x3 grid + arrow gap
  });

  return slots;
}

export const craftingTableGrid: CraftingGridConfig = {
  id: "crafting_table",
  name: "Crafting Table",
  icon: undefined,
  inputCols: 3,
  inputRows: 3,
  slots: buildSlots(),
  supportsShapeless: true,

  buildExportRequest(
    slotStates: Map<string, GridSlotState>,
    shapeless: boolean,
    count: number,
  ): ExportRecipeRequest {
    const inputs: RecipeSlot[] = [];
    const outputs: RecipeSlot[] = [];

    for (const [slotId, state] of slotStates) {
      if (!state.item) continue;

      const slot = this.slots.find((s) => s.id === slotId);
      if (!slot) continue;

      const recipeSlot: RecipeSlot = {
        slot_id: slotId,
        item_id: state.item.id,
        row: slot.row,
        col: slot.col,
      };

      if (slot.type === "input") {
        inputs.push(recipeSlot);
      } else {
        outputs.push(recipeSlot);
      }
    }

    return {
      grid_type: this.id,
      shapeless,
      inputs,
      outputs,
      count,
    };
  },
};
