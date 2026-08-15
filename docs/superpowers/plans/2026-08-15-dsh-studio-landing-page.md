# DS Studio Landing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a polished, responsive Chinese landing page for DS Studio 0.1.2 with a verified Windows x64 download link and a static `dist/` suitable for 1Panel.

**Architecture:** Keep the website isolated in `landing/` as dependency-free HTML, CSS, and JavaScript. A small Node.js build module copies deployable source and brand assets into `landing/dist/`; Node's built-in test runner verifies content, accessibility hooks, version consistency, and build output.

**Tech Stack:** Semantic HTML5, modern CSS, vanilla JavaScript, Node.js 24 built-in test runner and filesystem APIs.

---

## File map

- `landing/index.html`: semantic product content, navigation, CTAs, and workbench preview.
- `landing/styles.css`: deep-sea workbench visual system, responsive layout, focus states, and reduced-motion rules.
- `landing/app.js`: progressive reveal behavior with a no-JavaScript-safe baseline.
- `landing/package.json`: local test and production-build commands.
- `landing/scripts/build.mjs`: deterministic static build that recreates `dist/` and copies source plus brand assets.
- `landing/tests/site.test.mjs`: content, version, link, semantics, and styling requirements.
- `landing/tests/build.test.mjs`: build output and copied-asset requirements.
- `landing/README.md`: concise 1Panel build and deployment handoff.

### Task 1: Add failing landing-page contract tests

**Files:**
- Create: `landing/package.json`
- Create: `landing/tests/site.test.mjs`

- [ ] **Step 1: Add the test runner configuration**

```json
{
  "name": "dsh-studio-landing",
  "private": true,
  "version": "0.1.2",
  "type": "module",
  "scripts": {
    "test": "node --test tests/*.test.mjs",
    "build": "node scripts/build.mjs"
  }
}
```

- [ ] **Step 2: Write the failing page contract test**

```js
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);
const downloadUrl = "https://github.com/vayneluo/dsh-studio/releases/download/v0.1.2/dsh-studio_0.1.2_x64-setup.exe";

test("page presents DS Studio 0.1.2 and the verified installer", async () => {
  const html = await readFile(new URL("index.html", root), "utf8");
  assert.match(html, /DS Studio/);
  assert.match(html, /0\.1\.2/);
  assert.ok(html.includes(downloadUrl));
  assert.match(html, /Windows 11/);
});

test("page includes semantic navigation, product sections, and accessible CTAs", async () => {
  const html = await readFile(new URL("index.html", root), "utf8");
  for (const token of ["<header", "<main", "<footer", "id=\"features\"", "id=\"models\"", "id=\"download\""]) {
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
```

- [ ] **Step 3: Run the tests and verify RED**

Run: `Set-Location landing; npm test`

Expected: FAIL because `landing/index.html` and `landing/styles.css` do not exist.

### Task 2: Implement the semantic landing page and visual system

**Files:**
- Create: `landing/index.html`
- Create: `landing/styles.css`
- Create: `landing/app.js`
- Copy: `ui/logo.png` to `landing/assets/logo.png`
- Copy: `ui/brand.png` to `landing/assets/brand.png`

- [ ] **Step 1: Build the semantic page**

Create `index.html` with metadata for “DS Studio — DeepSeek Harness Windows 桌面工作台”, a skip link, sticky navigation, hero with the verified installer URL, a CSS-rendered desktop workbench preview, `features`, `models`, and `download` sections, and a footer linking to the repository and v0.1.2 release. All external links use `target="_blank" rel="noopener noreferrer"`.

- [ ] **Step 2: Implement the deep-sea workbench visual system**

Create `styles.css` using custom properties rooted in deep navy (`#050914`), ink (`#07111f`), electric blue (`#2f6bff`), cyan (`#18d9d0`), and warm white (`#f4f7fb`). Include an asymmetric hero grid, fine background grid, restrained glows, window chrome, model chips, feature rows, `:focus-visible`, breakpoints at 960px and 680px, and a `prefers-reduced-motion: reduce` override.

- [ ] **Step 3: Add progressive reveal behavior**

```js
document.documentElement.classList.add("has-js");

const observer = new IntersectionObserver((entries) => {
  for (const entry of entries) {
    if (!entry.isIntersecting) continue;
    entry.target.classList.add("is-visible");
    observer.unobserve(entry.target);
  }
}, { rootMargin: "0px 0px -10%", threshold: 0.08 });

document.querySelectorAll("[data-reveal]").forEach((element) => observer.observe(element));
```

- [ ] **Step 4: Copy the existing brand assets**

Run: `New-Item -ItemType Directory -Force landing/assets; Copy-Item ui/logo.png landing/assets/logo.png; Copy-Item ui/brand.png landing/assets/brand.png`

- [ ] **Step 5: Run the page tests and verify GREEN**

Run: `Set-Location landing; npm test`

Expected: all page contract tests pass.

### Task 3: Add and verify the deterministic production build

**Files:**
- Create: `landing/tests/build.test.mjs`
- Create: `landing/scripts/build.mjs`

- [ ] **Step 1: Write the failing build test**

```js
import assert from "node:assert/strict";
import { access, readFile, rm } from "node:fs/promises";
import test from "node:test";
import { buildSite } from "../scripts/build.mjs";

const root = new URL("../", import.meta.url);

test("buildSite creates a deployable dist with source and brand assets", async () => {
  await rm(new URL("dist/", root), { recursive: true, force: true });
  await buildSite();
  for (const path of ["dist/index.html", "dist/styles.css", "dist/app.js", "dist/assets/logo.png", "dist/assets/brand.png"]) {
    await access(new URL(path, root));
  }
  const html = await readFile(new URL("dist/index.html", root), "utf8");
  assert.doesNotMatch(html, /0\.1\.1/);
  assert.match(html, /0\.1\.2/);
});
```

- [ ] **Step 2: Run the test and verify RED**

Run: `Set-Location landing; node --test tests/build.test.mjs`

Expected: FAIL because `scripts/build.mjs` does not exist.

- [ ] **Step 3: Implement the build module**

```js
import { cp, mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
const output = new URL("dist/", root);
const files = ["index.html", "styles.css", "app.js", "assets"];

export async function buildSite() {
  await rm(output, { recursive: true, force: true });
  await mkdir(output, { recursive: true });
  for (const file of files) {
    await cp(new URL(file, root), new URL(`dist/${file}`, root), { recursive: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await buildSite();
  console.log("Built landing/dist");
}
```

- [ ] **Step 4: Run all tests and build**

Run: `Set-Location landing; npm test; npm run build`

Expected: tests pass and output ends with `Built landing/dist`.

### Task 4: Add deployment handoff and perform final verification

**Files:**
- Create: `landing/README.md`

- [ ] **Step 1: Document the 1Panel handoff**

Document these exact actions: run `npm test` and `npm run build` in `landing/`; create a 1Panel static website; upload the contents of `landing/dist/` as the site root; enable HTTPS; no reverse proxy or Node process is required.

- [ ] **Step 2: Run fresh verification**

Run: `Set-Location landing; npm test; npm run build; rg -n "0\.1\.1|href=\"#\"" index.html styles.css app.js README.md`

Expected: tests and build pass; `rg` produces no matches.

- [ ] **Step 3: Verify the release target**

Run: `curl.exe -sS -o NUL -w "%{http_code}" "https://github.com/vayneluo/dsh-studio/releases/download/v0.1.2/dsh-studio_0.1.2_x64-setup.exe"`

Expected: `302` or `200`.
