# DS Studio Interface Copy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the Windows title bar to `DS Studio` and replace the official bilingual home hero headline with the approved DS Studio copy.

**Architecture:** Keep the native label in Tauri configuration and add one focused Rust module for desktop-only DSH page copy. The module gates on the finished loopback DSH page and evaluates an idempotent script that changes only the headline beside the unchanged preview badge.

**Tech Stack:** Rust 2021, Tauri 2.11, WebView JavaScript, JSON configuration, Cargo tests

---

### Task 1: Rename the native window title

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Write a failing configuration test**

Add this test inside `src-tauri/src/lib.rs`'s existing `app_tests` module:

```rust
    #[test]
    fn main_window_uses_the_ds_studio_title() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["app"]["windows"][0]["title"], "DS Studio");
    }
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-wordmark-target app_tests::main_window_uses_the_ds_studio_title -- --nocapture`

Expected: FAIL because the configured title is `dsh-studio`.

- [ ] **Step 3: Change only the main window title**

In `src-tauri/tauri.conf.json`, change:

```json
{ "label": "main", "title": "DS Studio", "width": 1280, "height": 800 }
```

Do not change `productName`, `identifier`, `publisher`, bundle icons, or executable metadata.

- [ ] **Step 4: Run the focused test and verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-wordmark-target app_tests::main_window_uses_the_ds_studio_title -- --nocapture`

Expected: one focused test passes.

### Task 2: Replace the bilingual home headline

**Files:**
- Create: `src-tauri/src/interface_copy.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Register the module and write failing behavior tests**

Add `mod interface_copy;` with the module declarations in `src-tauri/src/lib.rs`. Create `src-tauri/src/interface_copy.rs` with tests for the intended private API:

```rust
#[cfg(test)]
mod tests {
    use super::{is_dsh_web_url, INTERFACE_COPY_SCRIPT};

    #[test]
    fn copy_runs_only_for_the_loopback_dsh_server() {
        assert!(is_dsh_web_url(
            &tauri::Url::parse("http://127.0.0.1:43123/").unwrap()
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("tauri://localhost/").unwrap()
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("https://127.0.0.1:43123/").unwrap()
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("http://localhost:43123/").unwrap()
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("http://127.0.0.1/").unwrap()
        ));
    }

    #[test]
    fn script_replaces_only_the_bilingual_preview_headline() {
        for expected in [
            "探索未至之境",
            "与 DS Studio 一起，探索未至之境",
            "Into the Unknown",
            "Explore the unknown with DS Studio",
            "预览版",
            "Preview",
            "__DS_STUDIO_COPY_OBSERVER__",
            "MutationObserver",
        ] {
            assert!(INTERFACE_COPY_SCRIPT.contains(expected), "missing {expected}");
        }
        assert!(!INTERFACE_COPY_SCRIPT.contains("welcomeTitle"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("BrandWordmark"));
    }
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-wordmark-target interface_copy::tests -- --nocapture`

Expected: compilation fails because `is_dsh_web_url` and `INTERFACE_COPY_SCRIPT` do not exist.

- [ ] **Step 3: Implement the gated idempotent headline script**

Above the tests in `src-tauri/src/interface_copy.rs`, implement:

```rust
use tauri::{
    webview::{PageLoadEvent, PageLoadPayload},
    Runtime, Webview,
};

const INTERFACE_COPY_SCRIPT: &str = r#"
(() => {
  const replacements = new Map([
    ['探索未至之境', '与 DS Studio 一起，探索未至之境'],
    ['Into the Unknown', 'Explore the unknown with DS Studio'],
  ]);
  const previewLabels = new Set(['预览版', 'Preview']);

  const applyCopy = () => {
    for (const headline of document.querySelectorAll('span')) {
      const source = headline.textContent?.trim();
      const replacement = replacements.get(source);
      if (!replacement || !headline.parentElement) continue;
      const siblingLabels = [...headline.parentElement.children]
        .map((element) => element.textContent?.trim());
      if (!siblingLabels.some((label) => previewLabels.has(label))) continue;
      headline.textContent = replacement;
    }
  };

  applyCopy();
  if (!window.__DS_STUDIO_COPY_OBSERVER__) {
    window.__DS_STUDIO_COPY_OBSERVER__ = new MutationObserver(applyCopy);
    window.__DS_STUDIO_COPY_OBSERVER__.observe(document.documentElement, {
      childList: true,
      characterData: true,
      subtree: true,
    });
  }
})();
"#;

fn is_dsh_web_url(url: &tauri::Url) -> bool {
    url.scheme() == "http" && url.host_str() == Some("127.0.0.1") && url.port().is_some()
}

pub(crate) fn handle_page_load<R: Runtime>(webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
    if matches!(payload.event(), PageLoadEvent::Finished) && is_dsh_web_url(payload.url()) {
        if let Err(error) = webview.eval(INTERFACE_COPY_SCRIPT) {
            eprintln!("failed to install DS Studio interface copy: {error}");
        }
    }
}
```

- [ ] **Step 4: Run the focused tests and verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-wordmark-target interface_copy::tests -- --nocapture`

Expected: both interface-copy tests pass.

- [ ] **Step 5: Register the finished-page callback**

In `src-tauri/src/lib.rs`, add:

```rust
        .invoke_handler(tauri::generate_handler![startup_ui_ready])
        .on_page_load(interface_copy::handle_page_load)
        .setup(|app| {
```

- [ ] **Step 6: Run formatting and the complete Rust suite**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml`

Run: `cargo test --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-wordmark-target -- --nocapture`

Expected: all tests pass, including the three new tests.

- [ ] **Step 7: Commit the implementation**

```powershell
git add -- src-tauri/tauri.conf.json src-tauri/src/interface_copy.rs src-tauri/src/lib.rs
git commit -m "feat: unify DS Studio interface copy"
```

### Task 3: Verify desktop behavior and unchanged assets

**Files:**
- Verify only: `src-tauri/icons/icon.png`
- Verify only: `src-tauri/icons/icon.ico`

- [ ] **Step 1: Run repository checks and build**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `git diff --check HEAD~1..HEAD`

Run: `cargo build --manifest-path src-tauri/Cargo.toml --target-dir C:\Temp\dsh-studio-wordmark-target`

Expected: every command succeeds.

- [ ] **Step 2: Launch an isolated verification instance**

Run with a temporary target directory and verification-only application identifier so the user's open app and data directory remain untouched:

```powershell
$env:CARGO_TARGET_DIR = 'C:\Temp\dsh-studio-wordmark-target'
cargo tauri dev --config '{"identifier":"com.dsh.studio.codex-copy-check-20260815"}'
```

Verify the title bar reads `DS Studio`; the hero reads `与 DS Studio 一起，探索未至之境`; the fish and `预览版` badge are unchanged; switching to English produces `Explore the unknown with DS Studio` and keeps `Preview` unchanged.

- [ ] **Step 3: Confirm icon resources are unchanged**

Run: `git diff HEAD~1..HEAD -- src-tauri/icons`

Expected: no output.

- [ ] **Step 4: Record final repository state**

Run: `git status --short`

Expected: no uncommitted implementation changes remain.
