# DS Studio Interface Copy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Present one consistent `DS Studio` product identity in the native title, startup guide, expanded sidebar, home hero, and bilingual first-run onboarding.

**Architecture:** Keep the native title in Tauri configuration, keep the startup-guide copy in the repository-owned static HTML, and keep loaded Web UI branding in the focused `interface_copy` module. The sidecar publishes its selected port into app state; only a finished page from that exact loopback origin receives an idempotent DOM script. The script targets stable structural and exact-copy anchors, observes React remounts and locale changes, and never patches bundled `node_modules` or broad-matches technical DeepSeek/DSH terms.

**Tech Stack:** Rust 2021, Tauri 2.11, WebView JavaScript, JSON configuration, Cargo tests, Node test runner

---

### Task 1: Unify the native startup guide

**Files:**
- Modify: `scripts/runtime-policy.test.mjs`
- Modify: `ui/index.html`

- [ ] **Step 1: Write a failing startup-copy test**

Add a Node test that requires `<title>DS Studio</title>`, `<h1>DS<br>STUDIO</h1>`, `正在启动 DS Studio…`, and `DS Studio 未能完成启动。`, while rejecting the old visible `DSH STUDIO` and DSH Web startup strings.

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
node --test --test-name-pattern="startup page uses DS Studio product copy" scripts\runtime-policy.test.mjs
```

Expected: FAIL because the existing HTML still contains `dsh-studio`, `DSH STUDIO`, and `正在启动官方 DSH Web…`.

- [ ] **Step 3: Replace only visible startup product copy**

In `ui/index.html`, set the document title to `DS Studio`, the two-line heading to `DS<br>STUDIO`, the loading status to `正在启动 DS Studio…`, and the failure status to `DS Studio 未能完成启动。`. Preserve the existing error content insertion via `textContent`.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run the same command and expect one passing test.

### Task 2: Extend the interface-copy contract tests

**Files:**
- Modify: `src-tauri/src/interface_copy.rs`

- [ ] **Step 1: Replace the old narrow script test with failing branding tests**

Keep the existing exact-origin URL test and replace `script_replaces_only_the_bilingual_preview_headline` with:

```rust
    #[test]
    fn script_unifies_the_requested_bilingual_branding() {
        for expected in [
            "探索未至之境",
            "与 DS Studio 一起，探索未至之境",
            "Into the Unknown",
            "Explore the unknown with DS Studio",
            "预览版",
            "Preview",
            "0 0 182 24",
            "data-ds-studio-wordmark",
            "DS Studio",
            "欢迎使用 DS Studio",
            "Welcome to DS Studio",
            "DS Studio 目前处于预览阶段",
            "DS Studio is currently in preview",
            "为 DS Studio 配置 DeepSeek 官方模型，即可开始使用。",
            "Configure the official DeepSeek provider for DS Studio to get started.",
            "__DS_STUDIO_COPY_OBSERVER__",
            "MutationObserver",
        ] {
            assert!(
                INTERFACE_COPY_SCRIPT.contains(expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn script_removes_only_the_hero_preview_badge_and_preserves_internal_names() {
        assert!(INTERFACE_COPY_SCRIPT.contains("badge.remove()"));
        assert!(INTERFACE_COPY_SCRIPT.contains("data-ds-studio-hero"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("querySelectorAll('*')"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("replaceAll('DeepSeek'"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("replaceAll('DSH'"));
    }
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml interface_copy::tests -- --nocapture
```

Expected: the exact-origin test passes, while the two new script-contract tests fail because the current script does not contain the sidebar, onboarding, or badge-removal behavior.

### Task 3: Implement precise, idempotent DS Studio branding

**Files:**
- Modify: `src-tauri/src/interface_copy.rs`

- [ ] **Step 1: Add exact bilingual copy maps and stable element markers**

Inside `INTERFACE_COPY_SCRIPT`, define the exact hero and onboarding transformations:

```javascript
const headlineReplacements = new Map([
  ['探索未至之境', '与 DS Studio 一起，探索未至之境'],
  ['Into the Unknown', 'Explore the unknown with DS Studio'],
]);
const brandedHeadlines = new Set(headlineReplacements.values());
const previewLabels = new Set(['预览版', 'Preview']);
const normalize = (value) => value?.replace(/\s+/g, ' ').trim();
const copyReplacements = new Map([
  ['内测声明', '欢迎使用 DS Studio'],
  ['Internal Testing Notice', 'Welcome to DS Studio'],
  [
    'DeepSeek Harness 目前的 0.1 版本仍处在面向 Harness 开发者进行测试的阶段，还有许多地方需要持续改进和打磨，希望听取广大开发者的反馈建议。预计 DeepSeek Harness 的核心插件以及基础 API 都会在接下来的一段时间内快速迭代、持续演化。',
    'DS Studio 目前处于预览阶段，产品体验和基础能力会持续改进，欢迎反馈建议。',
  ],
  [
    '我们期待与全球开发者一起，在开源、开放、可复用、可组合的基础设施之上，共同探索智能上限。欢迎全球 Harness 开发者加入 DSH 插件生态。',
    '我们期待与你一起，在开放、可复用、可组合的基础设施之上，共同探索智能上限。',
  ],
  [
    "DeepSeek Harness 0.1 remains in testing for Harness developers. Many areas need further improvement, and we welcome feedback from the developer community. DeepSeek Harness's core plugins and foundational APIs will continue to evolve rapidly over the coming months.",
    'DS Studio is currently in preview. The product experience and foundational capabilities will continue to improve, and we welcome your feedback.',
  ],
  [
    'We look forward to exploring the limits of intelligence with developers around the world, building on open-source, open, reusable, and composable infrastructure. We welcome Harness developers everywhere to join the DSH plugin ecosystem.',
    'We look forward to exploring the limits of intelligence with you on open, reusable, and composable infrastructure.',
  ],
  ['配置 DeepSeek 官方模型，即可开始使用。', '为 DS Studio 配置 DeepSeek 官方模型，即可开始使用。'],
  ['Configure the official DeepSeek provider to start building.', 'Configure the official DeepSeek provider for DS Studio to get started.'],
]);
```

- [ ] **Step 2: Replace only the expanded official wordmark with visible text**

Use the official wordmark's stable SVG view box rather than hashed CSS classes. Preserve the existing parent button so click, focus, and accessible behavior remain owned by the sidebar:

```javascript
const replaceWordmarks = (root) => {
  const selector = 'svg[viewBox="0 0 182 24"][aria-hidden="true"]';
  const wordmarks = root.matches?.(selector)
    ? [root]
    : [...root.querySelectorAll?.(selector) ?? []];
  for (const wordmark of wordmarks) {
    if (wordmark.parentElement?.tagName !== 'BUTTON') continue;
    const label = document.createElement('span');
    label.setAttribute('data-ds-studio-wordmark', '');
    label.textContent = 'DS Studio';
    Object.assign(label.style, {
      color: 'inherit',
      fontSize: '22px',
      fontWeight: '700',
      letterSpacing: '-0.02em',
      lineHeight: '1',
      whiteSpace: 'nowrap',
    });
    wordmark.replaceWith(label);
  }
};
```

- [ ] **Step 3: Brand the hero and remove only its direct preview sibling**

Mark a confirmed hero so a later locale text mutation remains scoped even after its badge has been removed:

```javascript
const replaceHeadline = (headline) => {
  if (!(headline instanceof HTMLSpanElement) || !headline.parentElement) return;
  const source = headline.textContent?.trim();
  const badges = [...headline.parentElement.children].filter((element) =>
    element !== headline && previewLabels.has(element.textContent?.trim())
  );
  const marked = headline.hasAttribute('data-ds-studio-hero');
  if (!marked && badges.length === 0) return;
  const replacement = headlineReplacements.get(source);
  if (!replacement && !brandedHeadlines.has(source)) return;
  headline.setAttribute('data-ds-studio-hero', '');
  if (replacement) headline.textContent = replacement;
  for (const badge of badges) badge.remove();
};
```

- [ ] **Step 4: Replace only exact leaf onboarding copy**

```javascript
const replaceProductCopy = (element) => {
  if (!(element instanceof HTMLElement) || element.childElementCount !== 0) return;
  if (!element.closest('[role="dialog"]')) return;
  const replacement = copyReplacements.get(normalize(element.textContent));
  if (replacement && element.textContent !== replacement) element.textContent = replacement;
};
```

- [ ] **Step 5: Apply transformations initially and after React mutations**

Update `applyWithin` to call all three transformations over the smallest relevant selector sets. Keep the single global observer marker and observe only `childList`, `characterData`, and `subtree`; do not observe attributes.

```javascript
const applyWithin = (root) => {
  if (!(root instanceof Element)) return;
  replaceWordmarks(root);
  if (root.matches('span')) replaceHeadline(root);
  for (const headline of root.querySelectorAll('span')) replaceHeadline(headline);
  if (root.matches('h1,h2,h3,p,div,span')) replaceProductCopy(root);
  for (const element of root.querySelectorAll('h1,h2,h3,p,div,span')) {
    replaceProductCopy(element);
  }
};
```

For `characterData` mutations, call `applyWithin(mutation.target.parentElement)`. For child additions, call it for the added element or its parent. This handles initial load, onboarding steps, React remounts, and locale changes without duplicating text or observers.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml interface_copy::tests -- --nocapture
```

Expected: all interface-copy tests pass.

### Task 4: Keep script execution on the exact sidecar origin

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Verify: `src-tauri/src/interface_copy.rs`

- [ ] **Step 1: Verify the exact-port regression test covers missing and mismatched ports**

The URL test must accept only `http://127.0.0.1:<published-port>/` and reject a mismatched port, `None`, default HTTP port, HTTPS, `localhost`, and `tauri://localhost`.

- [ ] **Step 2: Publish and clear the sidecar port in app state**

Use `AtomicU16 web_port` in `SidecarState`. Publish the selected port only after the sidecar is ready, pass `(port != 0).then_some(port)` into `interface_copy::handle_page_load`, and clear it when the sidecar is taken or navigation fails.

- [ ] **Step 3: Run the complete Rust and Node suites**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
node --test scripts\*.test.mjs
```

Expected: 35 or more Rust tests pass, all 21 Node tests pass, and formatting reports no diff.

### Task 5: Verify the real desktop UI and unchanged assets

**Files:**
- Verify only: `src-tauri/icons/icon.png`
- Verify only: `src-tauri/icons/icon.ico`

- [ ] **Step 1: Build an isolated verification binary**

Run:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-brand-target
```

Expected: build exits with code 0; the existing Windows linker stdout warning is allowed.

- [ ] **Step 2: Launch with an isolated identifier and first-run data**

Run a verification-only instance with a temporary Tauri identifier and data directory so the user's current app and profile are untouched.

Verify:

- title bar is `DS Studio`;
- native startup guide shows `DS STUDIO` and `正在启动 DS Studio…`;
- expanded sidebar shows only the prominent plain text `DS Studio`;
- first-run welcome title/body use DS Studio in Chinese and English;
- API-key onboarding description names DS Studio while keeping DeepSeek as the provider;
- hero copy is branded and `预览版` / `Preview` is absent;
- the home fish, collapsed-sidebar fish control, inputs, focus, and click behavior still work.

- [ ] **Step 3: Confirm icon resources and bundled packages are unchanged**

Run:

```powershell
git diff -- src-tauri/icons runtime/host/node_modules runtime/profile-seed
```

Expected: no output.

- [ ] **Step 4: Commit only the scoped implementation and documentation**

```powershell
git add -- ui/index.html scripts/runtime-policy.test.mjs src-tauri/src/interface_copy.rs src-tauri/src/lib.rs docs/superpowers/specs/2026-08-15-ds-studio-interface-copy-design.md docs/superpowers/plans/2026-08-15-ds-studio-interface-copy.md
git commit -m "feat: unify DS Studio branding"
```

Do not add `.superpowers/` or the independent provider-neutral onboarding plan.
