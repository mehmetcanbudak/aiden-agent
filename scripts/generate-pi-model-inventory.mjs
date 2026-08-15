#!/usr/bin/env node

// Regenerate the Rust runtime's release-pinned Pi model inventory from the
// exact JavaScript package used by the Electron v0.28.39 source. The emitted
// JSON is committed so production GPUI builds remain JavaScript-free.

import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageManifest = JSON.parse(
  await readFile(
    path.join(root, "node_modules/@earendil-works/pi-ai/package.json"),
    "utf8",
  ),
);

if (packageManifest.version !== "0.80.10") {
  throw new Error(
    `Expected @earendil-works/pi-ai 0.80.10, found ${String(packageManifest.version)}`,
  );
}

const { MODELS } = await import(
  pathToFileURL(
    path.join(
      root,
      "node_modules/@earendil-works/pi-ai/dist/models.generated.js",
    ),
  ).href
);

const inventory = Object.fromEntries(
  Object.entries(MODELS).map(([providerId, models]) => [providerId, Object.keys(models)]),
);

await writeFile(
  path.join(root, "resources/pi-models-0.80.10.json"),
  `${JSON.stringify(inventory, null, 2)}\n`,
  "utf8",
);
