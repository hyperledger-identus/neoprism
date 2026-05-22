import { join } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const SKILL_PATHS = [
  "skills/neoprism-release/SKILL.md",
];

export default function (pi: ExtensionAPI) {
  pi.on("resources_discover", async (event) => {
    const skillPaths = SKILL_PATHS.map((p) => join(event.cwd, p));
    return { skillPaths };
  });
}
