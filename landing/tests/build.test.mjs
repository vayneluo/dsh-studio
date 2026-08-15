import assert from "node:assert/strict";
import { access, readFile, rm } from "node:fs/promises";
import test from "node:test";
import { buildSite } from "../scripts/build.mjs";

const root = new URL("../", import.meta.url);

test("buildSite creates a deployable dist with source and brand assets", async () => {
  await rm(new URL("dist/", root), { recursive: true, force: true });
  await buildSite();

  for (const path of [
    "dist/index.html",
    "dist/styles.css",
    "dist/app.js",
    "dist/assets/logo.png",
    "dist/assets/brand.png",
  ]) {
    await access(new URL(path, root));
  }

  const html = await readFile(new URL("dist/index.html", root), "utf8");
  assert.doesNotMatch(html, /0\.1\.1/);
  assert.doesNotMatch(html, /0\.1\.2/);
  assert.match(html, /0\.1\.3/);
});
