import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

import { DragProvider, useDragContext } from "./hooks/useDragContext";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { ItemBrowser } from "./components/ItemBrowser/ItemBrowser";
import { CraftingArea } from "./components/CraftingArea/CraftingArea";
import { useMods } from "./hooks/useMods";
import { useCraftingGrid } from "./hooks/useCraftingGrid";
import {
  getDefaultGridId,
  getGrid,
} from "./components/CraftingArea/grids/registry";

import type { CraftingGridConfig } from "./components/CraftingArea/grids/types";

import "./App.css";

function AppInner() {
  const mods = useMods();
  const [activeGridId, setActiveGridId] = useState(getDefaultGridId());
  const activeConfig = getGrid(activeGridId)!;
  const grid = useCraftingGrid(activeConfig);
  const { getDragItem } = useDragContext();

  /** When an item is dropped on a grid slot, read it from the drag context */
  const handleDropSlot = useCallback(
    (slotId: string) => {
      const item = getDragItem();
      if (item) {
        grid.setSlot(slotId, item);
      }
    },
    [getDragItem, grid],
  );

  const handleGridChange = useCallback(
    (config: CraftingGridConfig) => {
      setActiveGridId(config.id);
      grid.resetForConfig(config);
    },
    [grid],
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
      }
    } catch (e) {
      console.error("Export failed:", e);
    }
  }, [grid, activeConfig]);

  const handleLoadMods = useCallback(async () => {
    await mods.loadMods();
  }, [mods]);

  return (
    <div className="app">
      <Toolbar
        modpackDir={mods.modpackDir}
        loading={mods.loading}
        onSelectModpackDir={mods.selectModpackDir}
        onLoadMods={handleLoadMods}
      />

      <div className="app__content">
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
