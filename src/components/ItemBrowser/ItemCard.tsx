import { memo, useCallback } from "react";
import type { ItemInfo } from "../../types/item";
import { useDragContext } from "../../hooks/useDragContext";

interface ItemCardProps {
  item: ItemInfo;
}

/**
 * A single item card in the item browser.
 * Uses pointer events for drag initiation — no per-element hook overhead,
 * and works reliably in Tauri's WebKitGTK webview (unlike HTML5 DnD).
 * Wrapped in React.memo to prevent re-renders when siblings change.
 */
export const ItemCard = memo(function ItemCard({ item }: ItemCardProps) {
  const { startDrag } = useDragContext();

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (e.button !== 0) return; // left-click only
      startDrag(item, e);
    },
    [item, startDrag],
  );

  return (
    <div
      className="item-card"
      onPointerDown={handlePointerDown}
      title={`${item.display_name}\n${item.id}`}
      style={{ touchAction: "none" }}
    >
      <div className="item-card__texture-wrapper">
        {item.texture_base64 ? (
          <img
            src={item.texture_base64}
            alt={item.display_name}
            className="item-card__texture"
            draggable={false}
          />
        ) : (
          <div className="item-card__placeholder" />
        )}
      </div>
      <span className="item-card__name">{item.display_name}</span>
    </div>
  );
});
