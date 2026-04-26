import { memo } from "react";
import type { RecipeFileEntry } from "../../types/recipe";
import type { LoadedMods, ItemInfo } from "../../types/item";

interface RecipeCardProps {
  entry: RecipeFileEntry;
  loadedMods: LoadedMods | null;
  isActive: boolean;
  onClick: () => void;
}

/**
 * A single recipe card in the Recipe Browser.
 * Shows the output item's texture (if loaded) as the preview icon,
 * the output item ID, and the recipe file name.
 */
export const RecipeCard = memo(function RecipeCard({
  entry,
  loadedMods,
  isActive,
  onClick,
}: RecipeCardProps) {
  const outputItemId = entry.recipe.output_item_id;

  // Try to resolve the output item texture from loaded mods
  const outputItem: ItemInfo | null = (() => {
    if (!loadedMods) return null;
    for (const items of Object.values(loadedMods.items)) {
      const found = items.find((i) => i.id === outputItemId);
      if (found) return found;
    }
    return null;
  })();

  const displayName = outputItem?.display_name ?? outputItemId;
  const shortName = outputItemId.includes(":")
    ? outputItemId.split(":")[1]
    : outputItemId;

  return (
    <button
      className={`recipe-card ${isActive ? "recipe-card--active" : ""}`}
      onClick={onClick}
      title={`${displayName}\n${entry.file_name}`}
    >
      <div className="recipe-card__icon">
        {outputItem?.texture_base64 ? (
          <img
            src={outputItem.texture_base64}
            alt={displayName}
            className="recipe-card__texture"
            draggable={false}
          />
        ) : (
          <div className="recipe-card__unknown-icon" title={outputItemId}>
            <span className="recipe-card__unknown-text">{shortName}</span>
          </div>
        )}
      </div>

      <div className="recipe-card__info">
        <span className="recipe-card__name">{displayName}</span>
        <span className="recipe-card__file">{entry.file_name}</span>
      </div>

      {entry.recipe.count > 1 && (
        <span className="recipe-card__count">x{entry.recipe.count}</span>
      )}
    </button>
  );
});
