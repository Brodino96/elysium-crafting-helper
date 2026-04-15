import { open } from "@tauri-apps/plugin-dialog";

interface ToolbarProps {
  modpackDir: string | null;
  loading: boolean;
  fromMods: boolean;
  hasCache: boolean;
  onSelectModpackDir: (path: string) => void;
  onLoadMods: () => void;
  onSetFromMods: (value: boolean) => void;
}

/**
 * Top toolbar: modpack folder picker, load button, and cache toggle.
 *
 * The "From Mods" checkbox controls whether loading re-parses all JARs
 * (checked) or uses the disk cache (unchecked). When no cache exists,
 * loading always parses from mods regardless of the checkbox.
 */
export function Toolbar({
  modpackDir,
  loading,
  fromMods,
  hasCache,
  onSelectModpackDir,
  onLoadMods,
  onSetFromMods,
}: ToolbarProps) {
  const handlePickModpackDir = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Modpack Directory",
    });
    if (selected) {
      onSelectModpackDir(selected);
    }
  };

  const truncatePath = (path: string, maxLen = 45) => {
    if (path.length <= maxLen) return path;
    return "..." + path.slice(path.length - maxLen);
  };

  const loadLabel = loading
    ? "Loading..."
    : fromMods || !hasCache
      ? "Load Mods"
      : "Load Cache";

  return (
    <div className="toolbar">
      <div className="toolbar__brand">
        <h1 className="toolbar__title">Elysium Crafting Helper</h1>
      </div>

      <div className="toolbar__folders">
        <button className="mc-btn" onClick={handlePickModpackDir}>
          {modpackDir ? truncatePath(modpackDir) : "Select Modpack Folder"}
        </button>

        <button
          className="mc-btn mc-btn--primary"
          onClick={onLoadMods}
          disabled={!modpackDir || loading}
        >
          {loadLabel}
        </button>

        <label className="mc-checkbox toolbar__from-mods">
          <input
            type="checkbox"
            checked={fromMods}
            onChange={(e) => onSetFromMods(e.target.checked)}
            disabled={loading}
          />
          From Mods
        </label>
      </div>
    </div>
  );
}
