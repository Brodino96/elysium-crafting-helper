import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { RecipeFileEntry } from "../types/recipe";

const STORAGE_KEY = "elysium_recipe_dir";

/**
 * Hook to manage recipe loading state.
 * Lets the user select a directory, then invokes the Rust backend to scan it
 * for crafting recipe JSON files.
 * Persists the last-used recipe directory in localStorage.
 */
export function useRecipes() {
  const [recipeDir, setRecipeDir] = useState<string | null>(
    () => localStorage.getItem(STORAGE_KEY),
  );
  const [recipes, setRecipes] = useState<RecipeFileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Open a native folder dialog and load recipes from the selected directory */
  const selectRecipeDir = useCallback(async () => {
    const selected = await open({
      title: "Select Recipe Directory",
      directory: true,
      multiple: false,
    });

    if (!selected) return; // user cancelled
    const dir = typeof selected === "string" ? selected : selected[0];

    setRecipeDir(dir);
    localStorage.setItem(STORAGE_KEY, dir);
    await loadRecipesFromDir(dir);
  }, []);

  /** Reload recipes from a directory path (internal helper) */
  const loadRecipesFromDir = useCallback(async (dir: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<RecipeFileEntry[]>("load_recipes_from_dir", {
        path: dir,
      });
      setRecipes(result);
    } catch (e) {
      setError(String(e));
      setRecipes([]);
    } finally {
      setLoading(false);
    }
  }, []);

  /** Re-scan the current recipe directory (e.g. after saving a new recipe) */
  const reloadRecipes = useCallback(async () => {
    if (!recipeDir) return;
    await loadRecipesFromDir(recipeDir);
  }, [recipeDir, loadRecipesFromDir]);

  return {
    recipeDir,
    recipes,
    loading,
    error,
    selectRecipeDir,
    reloadRecipes,
  };
}
