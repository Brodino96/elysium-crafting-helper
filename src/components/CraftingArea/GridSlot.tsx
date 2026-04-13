import { memo, useState, useCallback } from "react";
import type { GridSlotState } from "../../types/recipe";
import type { SlotDefinition } from "./grids/types";
import { useDragContext } from "../../hooks/useDragContext";

interface GridSlotProps {
  definition: SlotDefinition;
  state: GridSlotState;
  onDrop: (slotId: string) => void;
  onClear: (slotId: string) => void;
}

/**
 * A single slot in the crafting grid.
 * Uses pointer events for drop detection:
 *   - onPointerUp: if an item is being dragged, place it in this slot
 *   - onPointerEnter/Leave: highlight when an item is dragged over
 * Right-click (contextmenu) to clear.
 */
export const GridSlot = memo(function GridSlot({
  definition,
  state,
  onDrop,
  onClear,
}: GridSlotProps) {
  const [isOver, setIsOver] = useState(false);
  const { getDragItem } = useDragContext();

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      if (e.button !== 0) return; // left-click only
      if (getDragItem()) {
        onDrop(definition.id);
      }
      setIsOver(false);
    },
    [definition.id, onDrop, getDragItem],
  );

  const handlePointerEnter = useCallback(() => {
    // Only highlight when an item is being dragged
    if (getDragItem()) {
      setIsOver(true);
    }
  }, [getDragItem]);

  const handlePointerLeave = useCallback(() => {
    setIsOver(false);
  }, []);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      if (state.item) {
        onClear(definition.id);
      }
    },
    [state.item, definition.id, onClear],
  );

  const isOutput = definition.type === "output";

  return (
    <div
      className={`grid-slot ${isOutput ? "grid-slot--output" : "grid-slot--input"} ${isOver ? "grid-slot--over" : ""} ${state.item ? "grid-slot--filled" : ""}`}
      onPointerUp={handlePointerUp}
      onPointerEnter={handlePointerEnter}
      onPointerLeave={handlePointerLeave}
      onContextMenu={handleContextMenu}
      title={
        state.item
          ? `${state.item.display_name}\n${state.item.id}\nRight-click to remove`
          : isOutput
            ? "Output slot"
            : "Drop item here"
      }
    >
      {state.item ? (
        <div className="grid-slot__item">
          {state.item.texture_base64 ? (
            <img
              src={state.item.texture_base64}
              alt={state.item.display_name}
              className="grid-slot__texture"
              draggable={false}
            />
          ) : (
            <div className="grid-slot__placeholder-texture" />
          )}
        </div>
      ) : null}
      {definition.label && (
        <span className="grid-slot__label">{definition.label}</span>
      )}
    </div>
  );
});
