import type { CraftingGridConfig } from "./types";
import { craftingTableGrid } from "./crafting-table";

/**
 * Registry of all available crafting grid types.
 *
 * To add a new grid type:
 * 1. Create a new config file in this directory (e.g. furnace.ts)
 * 2. Import it here
 * 3. Add it to the gridRegistry map
 *
 * The UI will automatically pick up new grid types from this registry.
 */
const gridRegistry = new Map<string, CraftingGridConfig>();

// Register built-in grid types
gridRegistry.set(craftingTableGrid.id, craftingTableGrid);

// --- Add new grid types here ---
// import { furnaceGrid } from "./furnace";
// gridRegistry.set(furnaceGrid.id, furnaceGrid);
//
// import { smithingTableGrid } from "./smithing-table";
// gridRegistry.set(smithingTableGrid.id, smithingTableGrid);

/** Get all registered grid configurations */
export function getAllGrids(): CraftingGridConfig[] {
  return Array.from(gridRegistry.values());
}

/** Get a specific grid configuration by ID */
export function getGrid(id: string): CraftingGridConfig | undefined {
  return gridRegistry.get(id);
}

/** Get the default grid ID */
export function getDefaultGridId(): string {
  return craftingTableGrid.id;
}

export { gridRegistry };
