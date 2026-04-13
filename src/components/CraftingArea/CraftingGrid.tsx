import type { CraftingGridConfig } from "./grids/types";
import type { GridSlotState } from "../../types/recipe";
import { GridSlot } from "./GridSlot";

interface CraftingGridProps {
  config: CraftingGridConfig;
  slotStates: Map<string, GridSlotState>;
  onDropSlot: (slotId: string) => void;
  onClearSlot: (slotId: string) => void;
}

/**
 * Generic crafting grid renderer.
 * Reads a CraftingGridConfig and renders the appropriate grid layout.
 * This component is grid-type agnostic — it works with any registered config.
 */
export function CraftingGrid({
  config,
  slotStates,
  onDropSlot,
  onClearSlot,
}: CraftingGridProps) {
  const inputSlots = config.slots.filter((s) => s.type === "input");
  const outputSlots = config.slots.filter((s) => s.type === "output");

  return (
    <div className="crafting-grid">
      {/* Input grid */}
      <div
        className="crafting-grid__inputs"
        style={{
          display: "grid",
          gridTemplateColumns: `repeat(${config.inputCols}, 1fr)`,
          gridTemplateRows: `repeat(${config.inputRows}, 1fr)`,
        }}
      >
        {inputSlots.map((slot) => (
          <GridSlot
            key={slot.id}
            definition={slot}
            state={slotStates.get(slot.id) ?? { slotId: slot.id, item: null }}
            onDrop={onDropSlot}
            onClear={onClearSlot}
          />
        ))}
      </div>

      {/* Arrow separator */}
      <div className="crafting-grid__arrow">
        <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
          <path
            d="M4 16H24M24 16L18 10M24 16L18 22"
            stroke="#8b8b8b"
            strokeWidth="3"
            strokeLinecap="square"
          />
        </svg>
      </div>

      {/* Output slots */}
      <div className="crafting-grid__outputs">
        {outputSlots.map((slot) => (
          <GridSlot
            key={slot.id}
            definition={slot}
            state={slotStates.get(slot.id) ?? { slotId: slot.id, item: null }}
            onDrop={onDropSlot}
            onClear={onClearSlot}
          />
        ))}
      </div>
    </div>
  );
}
