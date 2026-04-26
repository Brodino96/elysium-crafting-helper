import { memo, useState, useCallback } from "react";
import type { GridSlotState } from "../../types/recipe";
import { isPlaceholder } from "../../types/recipe";
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
 *
 * Renders three states for a filled slot:
 *   1. Full ItemInfo with texture → shows the texture image
 *   2. Full ItemInfo without texture → shows the purple/black checkerboard
 *   3. PlaceholderItem → shows a grey slot with the item ID as text
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
  const item = state.item;
  const placeholder = item && isPlaceholder(item) ? item : null;
  const fullItem = item && !isPlaceholder(item) ? item : null;

  const titleText = item
    ? `${item.display_name}\n${item.id}\nRight-click to remove`
    : isOutput
      ? "Output slot"
      : "Drop item here";

  return (
    <div
      className={`grid-slot ${isOutput ? "grid-slot--output" : "grid-slot--input"} ${isOver ? "grid-slot--over" : ""} ${item ? "grid-slot--filled" : ""}`}
      onPointerUp={handlePointerUp}
      onPointerEnter={handlePointerEnter}
      onPointerLeave={handlePointerLeave}
      onContextMenu={handleContextMenu}
      title={titleText}
    >
      {fullItem && (
        <div className="grid-slot__item">
          {fullItem.texture_base64 ? (
            <img
              src={fullItem.texture_base64}
              alt={fullItem.display_name}
              className="grid-slot__texture"
              draggable={false}
            />
          ) : (
            <div className="grid-slot__placeholder-texture" />
          )}
        </div>
      )}

      {placeholder && (
        <div className="grid-slot__unknown-item" title={placeholder.id}>
          <span className="grid-slot__unknown-id">
            {placeholder.id.includes(":")
              ? placeholder.id.split(":")[1]
              : placeholder.id}
          </span>
        </div>
      )}

      {definition.label && (
        <span className="grid-slot__label">{definition.label}</span>
      )}
    </div>
  );
});

