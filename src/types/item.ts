/** Mirrors the Rust ModSource enum */
export type ModSource = "fabric_jar" | "kube_js";

/** Metadata about a single mod */
export interface ModInfo {
  id: string;
  name: string;
  version: string;
  source: ModSource;
}

/** A single game item with its metadata and texture */
export interface ItemInfo {
  id: string;
  mod_id: string;
  name: string;
  display_name: string;
  texture_base64: string | null;
  source: ModSource;
}

/** The complete result of loading all mods and items */
export interface LoadedMods {
  mods: ModInfo[];
  items: Record<string, ItemInfo[]>;
  total_items: number;
  warnings: string[];
}
