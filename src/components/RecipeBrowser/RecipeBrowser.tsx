import {
  useState,
  useMemo,
  useDeferredValue,
  useCallback,
  useRef,
} from "react";
import type { RecipeFileEntry } from "../../types/recipe";
import type { LoadedMods } from "../../types/item";
import { RecipeCard } from "./RecipeCard";

interface RecipeBrowserProps {
  recipes: RecipeFileEntry[];
  loading: boolean;
  recipeDir: string | null;
  loadedMods: LoadedMods | null;
  activeRecipePath: string | null;
  onSelectRecipe: (entry: RecipeFileEntry) => void;
  onSelectDir: () => void;
}

const MIN_WIDTH = 240;
const MAX_WIDTH = 600;
const DEFAULT_WIDTH = 300;

/**
 * Collapsible recipe browser panel, mirroring the ItemBrowser layout.
 * Shows all loaded recipe files with the output item as a preview icon.
 * Clicking a recipe populates the crafting grid.
 */
export function RecipeBrowser({
  recipes,
  loading,
  recipeDir,
  loadedMods,
  activeRecipePath,
  onSelectRecipe,
  onSelectDir,
}: RecipeBrowserProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const isResizing = useRef(false);

  // Resize handle logic (same pattern as ItemBrowser)
  const handleResizePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    isResizing.current = true;

    const handlePointerMove = (ev: PointerEvent) => {
      if (!isResizing.current) return;
      // The recipe browser is on the left; its right edge position equals clientX
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

  // Filter recipes by search query
  const filteredRecipes = useMemo(() => {
    const query = deferredSearch.toLowerCase().trim();
    if (!query) return recipes;
    return recipes.filter(
      (r) =>
        r.file_name.toLowerCase().includes(query) ||
        r.recipe.output_item_id.toLowerCase().includes(query),
    );
  }, [recipes, deferredSearch]);

  if (collapsed) {
    return (
      <div className="recipe-browser recipe-browser--collapsed">
        <button
          className="recipe-browser__collapse-btn"
          onClick={() => setCollapsed(false)}
          title="Expand Recipe Browser"
        >
          ▶
        </button>
      </div>
    );
  }

  return (
    <div
      className="recipe-browser"
      style={{ width, minWidth: MIN_WIDTH, maxWidth: MAX_WIDTH }}
    >
      {/* Header */}
      <div className="recipe-browser__header">
        <h2 className="recipe-browser__title">Recipes</h2>
        <div className="recipe-browser__header-actions">
          {recipes.length > 0 && (
            <span className="recipe-browser__count">
              {filteredRecipes.length}
              {deferredSearch ? ` / ${recipes.length}` : ""}
            </span>
          )}
          <button
            className="mc-btn mc-btn--sm"
            onClick={onSelectDir}
            title={recipeDir ?? "Select recipe directory"}
          >
            📂
          </button>
          <button
            className="recipe-browser__collapse-btn recipe-browser__collapse-btn--inline"
            onClick={() => setCollapsed(true)}
            title="Collapse Recipe Browser"
          >
            ◀
          </button>
        </div>
      </div>

      {/* Search */}
      {recipes.length > 0 && (
        <div className="recipe-browser__search">
          <input
            type="text"
            placeholder="Search recipes..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="mc-input"
          />
        </div>
      )}

      {/* Dir path hint */}
      {recipeDir && (
        <div className="recipe-browser__dir-hint" title={recipeDir}>
          {recipeDir.split(/[\\/]/).pop()}
        </div>
      )}

      {/* List */}
      <div className="recipe-browser__list">
        {loading && (
          <div className="recipe-browser__empty">Loading recipes...</div>
        )}

        {!loading && !recipeDir && (
          <div className="recipe-browser__empty">
            Click 📂 to select a recipe directory.
          </div>
        )}

        {!loading && recipeDir && recipes.length === 0 && (
          <div className="recipe-browser__empty">
            No crafting recipes found in this directory.
          </div>
        )}

        {!loading && recipes.length > 0 && filteredRecipes.length === 0 && (
          <div className="recipe-browser__empty">
            No recipes match your search.
          </div>
        )}

        {filteredRecipes.map((entry) => (
          <RecipeCard
            key={entry.file_path}
            entry={entry}
            loadedMods={loadedMods}
            isActive={activeRecipePath === entry.file_path}
            onClick={() => onSelectRecipe(entry)}
          />
        ))}
      </div>

      {/* Resize handle */}
      <div
        className="recipe-browser__resize-handle"
        onPointerDown={handleResizePointerDown}
      />
    </div>
  );
}
