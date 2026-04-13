import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LoadedMods } from "../types/item";

const STORAGE_KEY = "elysium_modpack_dir";

/**
 * Hook to manage mod loading state.
 * Communicates with the Rust backend to load mods from a modpack directory.
 * Persists the last-used modpack directory in localStorage so the user only
 * needs to click "Load Mods" on subsequent sessions.
 */
export function useMods() {
  const [loadedMods, setLoadedMods] = useState<LoadedMods | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [modpackDir, setModpackDir] = useState<string | null>(
    () => localStorage.getItem(STORAGE_KEY)
  );

  // On mount, if a path was saved from a previous session, restore it in the
  // Rust AppState so the backend is ready without requiring a re-select.
  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (!saved) return;
    invoke("set_modpack_dir", { path: saved }).catch(() => {
      // Saved path is no longer valid (e.g. directory was moved/deleted).
      // Clear it so the UI falls back to the normal select flow.
      localStorage.removeItem(STORAGE_KEY);
      setModpackDir(null);
    });
  }, []);

  /** Set the modpack root directory (must contain a mods/ subfolder) */
  const selectModpackDir = useCallback(async (path: string) => {
    try {
      setError(null);
      await invoke("set_modpack_dir", { path });
      setModpackDir(path);
      localStorage.setItem(STORAGE_KEY, path);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  /** Trigger mod loading / reloading */
  const loadMods = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<LoadedMods>("load_mods");
      setLoadedMods(result);
      return result;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    loadedMods,
    loading,
    error,
    modpackDir,
    selectModpackDir,
    loadMods,
  };
}
