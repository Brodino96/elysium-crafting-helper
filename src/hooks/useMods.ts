import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LoadedMods } from "../types/item";

/**
 * Hook to manage mod loading state.
 * Communicates with the Rust backend to load mods from a modpack directory.
 */
export function useMods() {
  const [loadedMods, setLoadedMods] = useState<LoadedMods | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [modpackDir, setModpackDir] = useState<string | null>(null);

  /** Set the modpack root directory (must contain a mods/ subfolder) */
  const selectModpackDir = useCallback(async (path: string) => {
    try {
      setError(null);
      await invoke("set_modpack_dir", { path });
      setModpackDir(path);
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
