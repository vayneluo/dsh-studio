import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);
const downloadUrl =
  "https://github.com/vayneluo/dsh-studio/releases/download/v0.1.2/dsh-studio_0.1.2_x64-setup.exe";

test("page presents DS Studio 0.1.2 and the verified installer", async () => {
  const html = await readFile(new URL("index.html", root), "utf8");

  assert.match(html, /DS Studio/);
  assert.match(html, /DeepSeek Harness，<br \/><em>Windows 桌面版。<\/em>/);
  assert.doesNotMatch(html, /把 Harness/);
  assert.match(html, /0\.1\.2/);
  assert.ok(html.includes(downloadUrl));
  assert.match(html, /Windows 11/);
});

test("page includes semantic navigation, product sections, and accessible CTAs", async () => {
  const html = await readFile(new URL("index.html", root), "utf8");

  for (const token of [
    "<header",
    "<main",
    "<footer",
    'id="features"',
    'id="models"',
    'id="download"',
  ]) {
    assert.ok(html.includes(token), `missing ${token}`);
  }

  assert.match(html, /class="skip-link"/);
  assert.match(html, /rel="noopener noreferrer"/);
});

test("styles include responsive, focus, and reduced-motion behavior", async () => {
  const css = await readFile(new URL("styles.css", root), "utf8");

  assert.match(css, /@media \(max-width:/);
  assert.match(css, /:focus-visible/);
  assert.match(css, /prefers-reduced-motion/);
});
