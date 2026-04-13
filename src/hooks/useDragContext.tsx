import {
  createContext,
  useContext,
  useCallback,
  useState,
  useEffect,
  useRef,
  useMemo,
} from "react";
import { createPortal } from "react-dom";
import type { ItemInfo } from "../types/item";

/**
 * Pointer-event-based drag context.
 *
 * Why not HTML5 drag-and-drop?
 *   → Tauri's WebKitGTK webview on Linux doesn't fire drop events reliably.
 *
 * Why not @dnd-kit?
 *   → It registers a useDraggable hook per element. With thousands of items,
 *     drag-start triggers expensive DOM measurements across all tracked nodes.
 *
 * This system:
 *   - ItemCard fires onPointerDown → startDrag(item, e)
 *   - Document-level pointermove updates a floating overlay (via direct DOM
 *     mutation on a ref, so no React re-renders per frame)
 *   - GridSlot fires onPointerUp → reads getDragItem(), places it
 *   - Document-level pointerup → endDrag() (fallback cleanup)
 *
 * The context value object is memoized with stable callbacks, so consumers
 * never re-render due to drag state changes. Only DragProvider itself
 * re-renders (twice: drag start + drag end) to mount/unmount the overlay portal.
 */

interface DragContextValue {
  /** Begin dragging an item. Call from onPointerDown on an item card. */
  startDrag: (item: ItemInfo, e: React.PointerEvent) => void;
  /** Read the item currently being dragged (ref-based, no re-render). */
  getDragItem: () => ItemInfo | null;
  /** End the current drag. Called automatically on document pointerup. */
  endDrag: () => void;
}

const DragCtx = createContext<DragContextValue | null>(null);

export function DragProvider({ children }: { children: React.ReactNode }) {
  const dragItemRef = useRef<ItemInfo | null>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const initialPosRef = useRef({ x: 0, y: 0 });

  // Only state we track: the item to render in the overlay (null = no drag).
  // This drives mount/unmount of the portal overlay.
  const [overlayItem, setOverlayItem] = useState<ItemInfo | null>(null);

  const endDrag = useCallback(() => {
    dragItemRef.current = null;
    setOverlayItem(null);
  }, []);

  const startDrag = useCallback(
    (item: ItemInfo, e: React.PointerEvent) => {
      if (e.button !== 0) return; // left-click only
      dragItemRef.current = item;
      initialPosRef.current = { x: e.clientX, y: e.clientY };
      setOverlayItem(item);
    },
    [],
  );

  const getDragItem = useCallback(() => dragItemRef.current, []);

  // Attach document-level listeners while dragging
  useEffect(() => {
    if (!overlayItem) return;

    const handlePointerMove = (e: PointerEvent) => {
      if (overlayRef.current) {
        overlayRef.current.style.left = `${e.clientX - 16}px`;
        overlayRef.current.style.top = `${e.clientY - 16}px`;
      }
    };

    // Fallback: any pointerup that wasn't consumed by a GridSlot clears drag.
    // GridSlot's onPointerUp fires first (event bubbling), reads the item,
    // then this handler cleans up.
    const handlePointerUp = () => {
      endDrag();
    };

    const preventSelect = (e: Event) => e.preventDefault();

    document.addEventListener("pointermove", handlePointerMove);
    document.addEventListener("pointerup", handlePointerUp);
    document.addEventListener("selectstart", preventSelect);

    return () => {
      document.removeEventListener("pointermove", handlePointerMove);
      document.removeEventListener("pointerup", handlePointerUp);
      document.removeEventListener("selectstart", preventSelect);
    };
  }, [overlayItem, endDrag]);

  // Stable context value — callbacks never change, so consumers never re-render
  const contextValue = useMemo(
    () => ({ startDrag, getDragItem, endDrag }),
    [startDrag, getDragItem, endDrag],
  );

  return (
    <DragCtx.Provider value={contextValue}>
      {children}
      {overlayItem &&
        createPortal(
          <div
            ref={overlayRef}
            className="drag-overlay"
            style={{
              position: "fixed",
              left: initialPosRef.current.x - 16,
              top: initialPosRef.current.y - 16,
              zIndex: 9999,
            }}
          >
            {overlayItem.texture_base64 ? (
              <img
                src={overlayItem.texture_base64}
                alt={overlayItem.display_name}
                className="drag-overlay__texture"
                draggable={false}
              />
            ) : (
              <div className="drag-overlay__placeholder" />
            )}
          </div>,
          document.body,
        )}
    </DragCtx.Provider>
  );
}

export function useDragContext(): DragContextValue {
  const ctx = useContext(DragCtx);
  if (!ctx) {
    throw new Error("useDragContext must be used inside DragProvider");
  }
  return ctx;
}
