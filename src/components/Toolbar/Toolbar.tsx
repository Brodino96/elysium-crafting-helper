import { open } from "@tauri-apps/plugin-dialog";

interface ToolbarProps {
  modpackDir: string | null;
  loading: boolean;
  onSelectModpackDir: (path: string) => void;
  onLoadMods: () => void;
}

/**
 * Top toolbar: modpack folder picker and load button.
 */
export function Toolbar({
  modpackDir,
  loading,
  onSelectModpackDir,
  onLoadMods,
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
          {loading ? "Loading..." : "Load Mods"}
        </button>
      </div>
    </div>
  );
}
