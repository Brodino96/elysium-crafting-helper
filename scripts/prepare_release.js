import { readFileSync, writeFileSync } from "node:fs"
import { resolve, dirname } from "node:path"
import { fileURLToPath } from "node:url"

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..")

const packageJson = JSON.parse(
  readFileSync(resolve(root, "package.json"), "utf-8")
)
const version = packageJson.version

if (!version) {
  console.error("No version found in package.json")
  process.exit(1)
}

console.log(`Syncing version ${version} to Tauri files...`)

// Update src-tauri/Cargo.toml
const cargoPath = resolve(root, "src-tauri/Cargo.toml")
const cargoContent = readFileSync(cargoPath, "utf-8")
const updatedCargo = cargoContent.replace(
  /^(version\s*=\s*)"[^"]*"/m,
  `$1"${version}"`
)
writeFileSync(cargoPath, updatedCargo, "utf-8")
console.log(`  Updated src-tauri/Cargo.toml`)

// Update src-tauri/tauri.conf.json
const tauriConfPath = resolve(root, "src-tauri/tauri.conf.json")
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf-8"))
tauriConf.version = version
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + "\n", "utf-8")
console.log(`  Updated src-tauri/tauri.conf.json`)

console.log("Done.")
