import { cp, mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
const output = new URL("dist/", root);
const files = ["index.html", "styles.css", "app.js", "assets"];

export async function buildSite() {
  await rm(output, { recursive: true, force: true });
  await mkdir(output, { recursive: true });

  for (const file of files) {
    await cp(new URL(file, root), new URL(`dist/${file}`, root), {
      recursive: true,
    });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await buildSite();
  console.log("Built landing/dist");
}
