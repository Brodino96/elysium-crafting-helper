import { useState, useCallback } from "react";
import type { ItemInfo } from "../types/item";
import type { LoadedMods } from "../types/item";
import type {
  GridSlotState,
  ExportRecipeRequest,
  PlaceholderItem,
  RecipeFileEntry,
} from "../types/recipe";
import type { CraftingGridConfig } from "../components/CraftingArea/grids/types";

/**
 * Hook to manage the state of a crafting grid.
 * Tracks which items are placed in which slots.
 */
export function useCraftingGrid(config: CraftingGridConfig) {
  const [slotStates, setSlotStates] = useState<Map<string, GridSlotState>>(
    () => {
      const initial = new Map<string, GridSlotState>();
      for (const slot of config.slots) {
        initial.set(slot.id, { slotId: slot.id, item: null });
      }
      return initial;
    },
  );

  const [shapeless, setShapeless] = useState(false);
  const [outputCount, setOutputCount] = useState(1);

  /** Place an item in a slot */
  const setSlot = useCallback(
    (slotId: string, item: ItemInfo | PlaceholderItem | null) => {
      setSlotStates((prev) => {
        const next = new Map(prev);
        next.set(slotId, { slotId, item });
        return next;
      });
    },
    [],
  );

  /** Clear a specific slot */
  const clearSlot = useCallback((slotId: string) => {
    setSlotStates((prev) => {
      const next = new Map(prev);
      next.set(slotId, { slotId, item: null });
      return next;
    });
  }, []);

  /** Clear all slots */
  const clearAll = useCallback(() => {
    setSlotStates((prev) => {
      const next = new Map<string, GridSlotState>();
      for (const [slotId] of prev) {
        next.set(slotId, { slotId, item: null });
      }
      return next;
    });
  }, []);

  /** Reset slots when grid config changes */
  const resetForConfig = useCallback((newConfig: CraftingGridConfig) => {
    const initial = new Map<string, GridSlotState>();
    for (const slot of newConfig.slots) {
      initial.set(slot.id, { slotId: slot.id, item: null });
    }
    setSlotStates(initial);
    setShapeless(false);
    setOutputCount(1);
  }, []);

  /**
   * Populate the grid from a recipe file entry.
   * Items that exist in loadedMods are resolved to their full ItemInfo;
   * unknown items become PlaceholderItems so the slot still renders something.
   *
   * `loadedMods` may be null (mods not loaded yet) — in that case every item
   * becomes a placeholder.
   */
  const loadRecipe = useCallback(
    (entry: RecipeFileEntry, loadedMods: LoadedMods | null) => {
      const { recipe } = entry;

      // Build a flat lookup: item_id → ItemInfo
      const itemLookup = new Map<string, ItemInfo>();
      if (loadedMods) {
        for (const items of Object.values(loadedMods.items)) {
          for (const item of items) {
            itemLookup.set(item.id, item);
          }
        }
      }

      const resolveItem = (itemId: string): ItemInfo | PlaceholderItem => {
        const found = itemLookup.get(itemId);
        if (found) return found;
        return {
          _placeholder: true,
          id: itemId,
          display_name: itemId,
        } satisfies PlaceholderItem;
      };

      // Start from a fresh grid
      const next = new Map<string, GridSlotState>();
      for (const slot of config.slots) {
        next.set(slot.id, { slotId: slot.id, item: null });
      }

      // Fill input slots
      for (const inputSlot of recipe.inputs) {
        const slotId = `input_${inputSlot.row}_${inputSlot.col}`;
        if (next.has(slotId)) {
          next.set(slotId, { slotId, item: resolveItem(inputSlot.item_id) });
        }
      }

      // Fill output slot
      const outputSlotId = "output_0";
      if (next.has(outputSlotId)) {
        next.set(outputSlotId, {
          slotId: outputSlotId,
          item: resolveItem(recipe.output_item_id),
        });
      }

      setSlotStates(next);
      setShapeless(recipe.recipe_type === "shapeless");
      setOutputCount(recipe.count);
    },
    [config],
  );

  /** Build export request from current state */
  const buildExportRequest = useCallback((): ExportRecipeRequest => {
    return config.buildExportRequest(slotStates, shapeless, outputCount);
  }, [config, slotStates, shapeless, outputCount]);

  /** Check if the grid has any items placed */
  const hasItems = useCallback((): boolean => {
    for (const [, state] of slotStates) {
      if (state.item) return true;
    }
    return false;
  }, [slotStates]);

  /** Check if the grid has both inputs and outputs */
  const isComplete = useCallback((): boolean => {
    const inputSlotIds = config.slots
      .filter((s) => s.type === "input")
      .map((s) => s.id);
    const outputSlotIds = config.slots
      .filter((s) => s.type === "output")
      .map((s) => s.id);

    const hasInput = inputSlotIds.some(
      (id) => slotStates.get(id)?.item != null,
    );
    const hasOutput = outputSlotIds.some(
      (id) => slotStates.get(id)?.item != null,
    );

    return hasInput && hasOutput;
  }, [config, slotStates]);

  return {
    slotStates,
    shapeless,
    outputCount,
    setSlot,
    clearSlot,
    clearAll,
    resetForConfig,
    loadRecipe,
    setShapeless,
    setOutputCount,
    buildExportRequest,
    hasItems,
    isComplete,
  };
}

