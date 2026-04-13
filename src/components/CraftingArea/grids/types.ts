import type { GridSlotState, ExportRecipeRequest } from "../../../types/recipe";

/**
 * Defines a single slot in a crafting grid layout.
 */
export interface SlotDefinition {
  /** Unique slot identifier, e.g. "input_0_0" or "output_0" */
  id: string;
  /** Whether this is an input or output slot */
  type: "input" | "output";
  /** Row position in the grid (0-indexed) */
  row: number;
  /** Column position in the grid (0-indexed) */
  col: number;
  /** Optional display label */
  label?: string;
}

/**
 * Configuration for a crafting grid type.
 *
 * This is the core abstraction for modularity — each crafting type
 * (crafting table, furnace, smithing table, etc.) is just a different
 * CraftingGridConfig registered in the registry.
 *
 * To add a new crafting type:
 * 1. Create a new file in this directory (e.g. furnace.ts)
 * 2. Export a CraftingGridConfig
 * 3. Register it in registry.ts
 */
export interface CraftingGridConfig {
  /** Unique identifier for this grid type */
  id: string;
  /** Human-readable name shown in the UI */
  name: string;
  /** Optional icon (emoji or path) */
  icon?: string;
  /** Number of input columns in the grid layout */
  inputCols: number;
  /** Number of input rows in the grid layout */
  inputRows: number;
  /** All slot definitions (inputs + outputs) */
  slots: SlotDefinition[];
  /** Whether this grid type supports a shapeless toggle */
  supportsShapeless: boolean;
  /**
   * Build an ExportRecipeRequest from the current grid state.
   * Each grid type knows how to serialize its own state.
   */
  buildExportRequest: (
    slotStates: Map<string, GridSlotState>,
    shapeless: boolean,
    count: number,
  ) => ExportRecipeRequest;
}
