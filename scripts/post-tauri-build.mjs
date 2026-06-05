import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

if (process.platform !== "linux") {
  process.exit(0);
}

const script = join(root, "scripts", "install-desktop-shortcut.sh");
const result = spawnSync(script, { stdio: "inherit", cwd: root });

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
