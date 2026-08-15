# DS Studio App Brand Assets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the supplied DS logo and DS Studio wordmark to the Windows executable and the DSH Web header in v0.1.2.

**Architecture:** Generate the standard Tauri icon set from `ui/logo.png`. Embed `ui/brand.png` into the Rust binary and inject it into the loopback DSH page as a self-contained data URL, reusing the existing page-load and mutation-observer boundary.

**Tech Stack:** Node.js built-in test runner, Tauri CLI 2.x, Rust, base64 0.22, DOM injection JavaScript.

---

### Task 1: Lock the supplied assets and reject the old icon

**Files:**
- Create: `scripts/brand-assets.test.mjs`
- Modify: `src-tauri/src/interface_copy.rs`

- [ ] **Step 1: Add a failing asset contract test**

```js
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('../', import.meta.url)
const sha256 = async (path) => createHash('sha256').update(await readFile(new URL(path, root))).digest('hex')

test('supplied brand inputs remain canonical', async () => {
  assert.equal(await sha256('ui/logo.png'), 'd12408a032c5ec00d39f4cc050f3451de04dc90279045a8070dbd2880bdb6b84')
  assert.equal(await sha256('ui/brand.png'), 'e9ed4eeb9a4bcc4433b8977657d7589e3e78ad8ac359c6f86e9d1833f1276594')
})

test('generated app icon no longer uses the legacy D artwork', async () => {
  assert.notEqual(await sha256('src-tauri/icons/icon.png'), '506bd781a772e4651df1322b0d726ec1a7d7909a000e5236db6dd3c6021d972d')
  assert.notEqual(await sha256('src-tauri/icons/icon.ico'), 'c7b8b8af986fbf16be050bf491e2e703592fd4da85f9e2fe52b8637ffdc0d974')
})
```

- [ ] **Step 2: Extend the Rust branding test before implementation**

Add assertions for `include_bytes!("../../ui/brand.png")`, `data:image/png;base64`, `createElement('img')`, `alt = 'DS Studio'`, `objectFit: 'cover'`, and the absence of `label.textContent = 'DS Studio'`.

- [ ] **Step 3: Run tests and verify RED**

Run: `node --test scripts/brand-assets.test.mjs` and `cargo test --manifest-path src-tauri/Cargo.toml interface_copy`

Expected: the asset test rejects the legacy icon and the Rust test rejects the text-only wordmark implementation.

### Task 2: Generate the Windows icon set

**Files:**
- Modify: `src-tauri/icons/32x32.png`
- Modify: `src-tauri/icons/128x128.png`
- Modify: `src-tauri/icons/128x128@2x.png`
- Modify: `src-tauri/icons/icon.png`
- Modify: `src-tauri/icons/icon.ico`

- [ ] **Step 1: Generate icons from the canonical logo**

Run from `src-tauri`: `cargo tauri icon ../ui/logo.png --output icons`

Expected: Tauri rewrites the standard PNG and ICO assets from the DS monogram source.

- [ ] **Step 2: Run the asset test and verify GREEN**

Run: `node --test scripts/brand-assets.test.mjs`

Expected: both asset contract tests pass.

### Task 3: Replace the text wordmark with the embedded brand image

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/interface_copy.rs`

- [ ] **Step 1: Add base64 encoding support**

Add `base64 = "0.22"` to `[dependencies]` and refresh the lockfile with `cargo check --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 2: Embed and assemble the script**

Use `include_bytes!("../../ui/brand.png")`, `OnceLock<String>`, and `base64::engine::general_purpose::STANDARD` to replace a single `__DS_STUDIO_BRAND_DATA_URL__` marker in the script template.

- [ ] **Step 3: Inject the brand image**

Replace the text span with an image whose source is the embedded data URL, alternative text is `DS Studio`, dimensions are 182×24, and styles include `objectFit: 'cover'`, `objectPosition: '50% 50%'`, a white background, and a small border radius.

- [ ] **Step 4: Run the Rust test and verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml interface_copy`

Expected: interface copy tests pass, including the image-brand assertions.

### Task 4: Verify, build, and launch v0.1.2

**Files:**
- Verify only: `scripts/*.test.mjs`, `src-tauri/`

- [ ] **Step 1: Run project gates**

Run the new brand test, the existing profile/runtime/provider tests, smoke test, and full Rust suite.

- [ ] **Step 2: Build the desktop app**

Run from `src-tauri`: `cargo tauri build`

Expected: a v0.1.2 executable and NSIS installer are generated without bundle errors.

- [ ] **Step 3: Launch the executable visibly**

Start `src-tauri/target/release/dsh-studio.exe` as a visible process so the Windows icon and injected DSH header brand can be inspected.
