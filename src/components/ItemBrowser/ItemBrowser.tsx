import { useState, useMemo, useDeferredValue, useCallback, useRef, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { LoadedMods, ItemInfo, ModInfo } from "../../types/item";
import { ModGroup } from "./ModGroup";

interface ItemBrowserProps {
  loadedMods: LoadedMods | null;
  loading: boolean;
}

const MIN_WIDTH = 240;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 360;

/**
 * Left panel: search bar + virtualized scrollable list of items grouped by mod.
 * Right edge is draggable to resize.
 *
 * Uses @tanstack/react-virtual to only render mod groups visible in the
 * scroll viewport. Each group's actual height is measured via ResizeObserver
 * (so expanding/collapsing a group updates layout automatically).
 */
export function ItemBrowser({ loadedMods, loading }: ItemBrowserProps) {
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const isResizing = useRef(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Resize handle logic
  const handleResizePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    isResizing.current = true;

    const handlePointerMove = (ev: PointerEvent) => {
      if (!isResizing.current) return;
      const newWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, ev.clientX));
      setWidth(newWidth);
    };

    const handlePointerUp = () => {
      isResizing.current = false;
      document.removeEventListener("pointermove", handlePointerMove);
      document.removeEventListener("pointerup", handlePointerUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.addEventListener("pointermove", handlePointerMove);
    document.addEventListener("pointerup", handlePointerUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  // Filter items based on search
  const filteredGroups = useMemo(() => {
    if (!loadedMods) return [];

    const query = deferredSearch.toLowerCase().trim();
    const groups: { mod: ModInfo; items: ItemInfo[] }[] = [];

    for (const mod of loadedMods.mods) {
      const modItems = loadedMods.items[mod.id] ?? [];

      const filtered = query
        ? modItems.filter(
            (item) =>
              item.display_name.toLowerCase().includes(query) ||
              item.id.toLowerCase().includes(query),
          )
        : modItems;

      if (filtered.length > 0) {
        groups.push({ mod, items: filtered });
      }
    }

    return groups;
  }, [loadedMods, deferredSearch]);

  const totalFiltered = filteredGroups.reduce(
    (sum, g) => sum + g.items.length,
    0,
  );

  // Virtualizer — only renders groups visible in the scroll viewport.
  // estimateSize is the collapsed header height; measureElement handles
  // actual sizes (expanded groups are taller).
  const virtualizer = useVirtualizer({
    count: filteredGroups.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 32,
    overscan: 3,
    getItemKey: (index) => filteredGroups[index]?.mod.id ?? String(index),
  });

  // Reset scroll position when search query changes so the user
  // always starts from the top of the filtered results.
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [deferredSearch]);

  const hasGroups = filteredGroups.length > 0;

  return (
    <div className="item-browser" style={{ width, minWidth: MIN_WIDTH, maxWidth: MAX_WIDTH }}>
      <div className="item-browser__header">
        <h2 className="item-browser__title">Items</h2>
        {loadedMods && (
          <span className="item-browser__count">
            {totalFiltered}
            {deferredSearch ? ` / ${loadedMods.total_items}` : ""}
          </span>
        )}
      </div>

      <div className="item-browser__search">
        <input
          type="text"
          placeholder="Search items..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="mc-input"
        />
      </div>

      <div ref={scrollRef} className="item-browser__list">
        {loading && (
          <div className="item-browser__loading">Loading mods...</div>
        )}

        {!loading && !loadedMods && (
          <div className="item-browser__empty">
            Select a mods folder to begin.
          </div>
        )}

        {!loading && loadedMods && !hasGroups && (
          <div className="item-browser__empty">
            {deferredSearch ? "No items match your search." : "No items found."}
          </div>
        )}

        {hasGroups && (
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((virtualRow) => {
              const { mod, items } = filteredGroups[virtualRow.index];
              return (
                <div
                  key={mod.id}
                  data-index={virtualRow.index}
                  ref={virtualizer.measureElement}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <ModGroup modName={mod.name} items={items} forceExpanded={deferredSearch.trim().length > 0} />
                </div>
              );
            })}
          </div>
        )}
      </div>

      {loadedMods && loadedMods.warnings.length > 0 && (
        <div className="item-browser__warnings">
          {loadedMods.warnings.map((w, i) => (
            <div key={i} className="item-browser__warning">
              {w}
            </div>
          ))}
        </div>
      )}

      {/* Resize handle */}
      <div
        className="item-browser__resize-handle"
        onPointerDown={handleResizePointerDown}
      />
    </div>
  );
}
