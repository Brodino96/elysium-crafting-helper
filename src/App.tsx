import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

import { DragProvider, useDragContext } from "./hooks/useDragContext";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { ItemBrowser } from "./components/ItemBrowser/ItemBrowser";
import { CraftingArea } from "./components/CraftingArea/CraftingArea";
import { RecipeBrowser } from "./components/RecipeBrowser/RecipeBrowser";
import { useMods } from "./hooks/useMods";
import { useCraftingGrid } from "./hooks/useCraftingGrid";
import { useRecipes } from "./hooks/useRecipes";
import {
  getDefaultGridId,
  getGrid,
} from "./components/CraftingArea/grids/registry";

import type { CraftingGridConfig } from "./components/CraftingArea/grids/types";
import type { RecipeFileEntry } from "./types/recipe";

import "./App.css";

function AppInner() {
  const mods = useMods();
  const recipes = useRecipes();
  const [activeGridId, setActiveGridId] = useState(getDefaultGridId());
  const activeConfig = getGrid(activeGridId)!;
  const grid = useCraftingGrid(activeConfig);
  const { getDragItem } = useDragContext();

  /** Track which recipe is currently loaded in the grid */
  const [activeRecipePath, setActiveRecipePath] = useState<string | null>(null);

  /** When an item is dropped on a grid slot, read it from the drag context */
  const handleDropSlot = useCallback(
    (slotId: string) => {
      const item = getDragItem();
      if (item) {
        grid.setSlot(slotId, item);
        // User modified the grid manually — deselect the active recipe
        setActiveRecipePath(null);
      }
    },
    [getDragItem, grid],
  );

  const handleGridChange = useCallback(
    (config: CraftingGridConfig) => {
      setActiveGridId(config.id);
      grid.resetForConfig(config);
      setActiveRecipePath(null);
    },
    [grid],
  );

  /** Load a recipe from the recipe browser into the crafting grid */
  const handleSelectRecipe = useCallback(
    (entry: RecipeFileEntry) => {
      grid.loadRecipe(entry, mods.loadedMods);
      setActiveRecipePath(entry.file_path);
    },
    [grid, mods.loadedMods],
  );

  const handleExport = useCallback(async () => {
    if (!grid.isComplete()) return;

    try {
      const request = grid.buildExportRequest();
      const json = await invoke<string>("export_recipe", { request });

      // Derive filename from the output item id (e.g. "minecraft:dirt" → "dirt.json")
      const outputSlot = activeConfig.slots.find((s) => s.type === "output");
      const outputItem = outputSlot
        ? grid.slotStates.get(outputSlot.id)?.item
        : null;
      const defaultName = outputItem
        ? outputItem.id.split(":").pop() + ".json"
        : "recipe.json";

      const path = await save({
        title: "Save Recipe",
        defaultPath: defaultName,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (path) {
        await invoke("save_file", { path, content: json });
        // Refresh the recipe browser in case the file was saved to the loaded dir
        recipes.reloadRecipes();
      }
    } catch (e) {
      console.error("Export failed:", e);
    }
  }, [grid, activeConfig, recipes]);

  const handleLoadMods = useCallback(async () => {
    await mods.loadMods();
  }, [mods]);

  return (
    <div className="app">
      <Toolbar
        modpackDir={mods.modpackDir}
        loading={mods.loading}
        fromMods={mods.fromMods}
        hasCache={mods.hasCache}
        onSelectModpackDir={mods.selectModpackDir}
        onLoadMods={handleLoadMods}
        onSetFromMods={mods.setFromMods}
      />

      <div className="app__content">
        <RecipeBrowser
          recipes={recipes.recipes}
          loading={recipes.loading}
          recipeDir={recipes.recipeDir}
          loadedMods={mods.loadedMods}
          activeRecipePath={activeRecipePath}
          onSelectRecipe={handleSelectRecipe}
          onSelectDir={recipes.selectRecipeDir}
        />

        <ItemBrowser loadedMods={mods.loadedMods} loading={mods.loading} />

        <CraftingArea
          slotStates={grid.slotStates}
          shapeless={grid.shapeless}
          outputCount={grid.outputCount}
          onDropSlot={handleDropSlot}
          onClearSlot={grid.clearSlot}
          onClearAll={grid.clearAll}
          onSetShapeless={grid.setShapeless}
          onSetOutputCount={grid.setOutputCount}
          onExport={handleExport}
          onGridChange={handleGridChange}
          activeGridId={activeGridId}
          canExport={grid.isComplete()}
        />
      </div>

      {mods.error && <div className="app__error">{mods.error}</div>}
      {recipes.error && <div className="app__error">{recipes.error}</div>}
    </div>
  );
}

function App() {
  return (
    <DragProvider>
      <AppInner />
    </DragProvider>
  );
}

export default App;

