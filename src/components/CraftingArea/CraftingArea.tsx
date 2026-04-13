import { CraftingGrid } from "./CraftingGrid";
import { getAllGrids, getGrid } from "./grids/registry";
import type { CraftingGridConfig } from "./grids/types";
import type { GridSlotState } from "../../types/recipe";

interface CraftingAreaProps {
  slotStates: Map<string, GridSlotState>;
  shapeless: boolean;
  outputCount: number;
  onDropSlot: (slotId: string) => void;
  onClearSlot: (slotId: string) => void;
  onClearAll: () => void;
  onSetShapeless: (shapeless: boolean) => void;
  onSetOutputCount: (count: number) => void;
  onExport: () => void;
  onGridChange: (config: CraftingGridConfig) => void;
  activeGridId: string;
  canExport: boolean;
}

/**
 * Right panel: grid type selector + active crafting grid + controls.
 */
export function CraftingArea({
  slotStates,
  shapeless,
  outputCount,
  onDropSlot,
  onClearSlot,
  onClearAll,
  onSetShapeless,
  onSetOutputCount,
  onExport,
  onGridChange,
  activeGridId,
  canExport,
}: CraftingAreaProps) {
  const allGrids = getAllGrids();
  const activeConfig = getGrid(activeGridId);

  if (!activeConfig) {
    return <div className="crafting-area">No grid configuration found.</div>;
  }

  return (
    <div className="crafting-area">
      <div className="crafting-area__header">
        <h2 className="crafting-area__title">{activeConfig.name}</h2>

        {/* Grid type selector (only show if multiple types available) */}
        {allGrids.length > 1 && (
          <div className="crafting-area__grid-selector">
            {allGrids.map((grid) => (
              <button
                key={grid.id}
                className={`mc-btn mc-btn--sm ${grid.id === activeGridId ? "mc-btn--active" : ""}`}
                onClick={() => onGridChange(grid)}
              >
                {grid.icon && <span>{grid.icon}</span>}
                {grid.name}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* The crafting grid */}
      <CraftingGrid
        config={activeConfig}
        slotStates={slotStates}
        onDropSlot={onDropSlot}
        onClearSlot={onClearSlot}
      />

      {/* Controls */}
      <div className="crafting-area__controls">
        {activeConfig.supportsShapeless && (
          <label className="mc-checkbox">
            <input
              type="checkbox"
              checked={shapeless}
              onChange={(e) => onSetShapeless(e.target.checked)}
            />
            <span>Shapeless</span>
          </label>
        )}

        <label className="mc-number">
          <span>Count:</span>
          <input
            type="number"
            min={1}
            max={64}
            value={outputCount}
            onChange={(e) =>
              onSetOutputCount(Math.max(1, Math.min(64, parseInt(e.target.value) || 1)))
            }
          />
        </label>

        <div className="crafting-area__actions">
          <button className="mc-btn mc-btn--danger" onClick={onClearAll}>
            Clear
          </button>
          <button
            className="mc-btn mc-btn--primary"
            onClick={onExport}
            disabled={!canExport}
          >
            Export Recipe
          </button>
        </div>
      </div>
    </div>
  );
}
