import { memo, useState } from "react";
import type { ItemInfo } from "../../types/item";
import { ItemCard } from "./ItemCard";

interface ModGroupProps {
  modName: string;
  items: ItemInfo[];
  /** When true, the group is forced open (e.g. during search). */
  forceExpanded?: boolean;
}

/**
 * A collapsible group of items belonging to a single mod.
 */
export const ModGroup = memo(function ModGroup({ modName, items, forceExpanded }: ModGroupProps) {
  const [collapsed, setCollapsed] = useState(true);

  // If forceExpanded is set (search is active), override the collapsed state.
  const isCollapsed = forceExpanded ? false : collapsed;

  return (
    <div className="mod-group">
      <button
        className="mod-group__header"
        onClick={() => setCollapsed(!collapsed)}
      >
        <span className={`mod-group__arrow ${isCollapsed ? "" : "mod-group__arrow--open"}`}>
          &#9654;
        </span>
        <span className="mod-group__name">{modName}</span>
        <span className="mod-group__count">{items.length}</span>
      </button>

      {!isCollapsed && (
        <div className="mod-group__items">
          {items.map((item) => (
            <ItemCard key={item.id} item={item} />
          ))}
        </div>
      )}
    </div>
  );
});
