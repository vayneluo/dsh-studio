# DS Studio Sidebar Wordmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the expanded DSH sidebar's official SVG wordmark with a prominent text-only `DS Studio` wordmark while preserving the desktop icon and sidebar behavior.

**Architecture:** Add one focused Rust module that owns URL gating, the idempotent browser injection script, and the Tauri page-load callback. Register that callback on the existing Tauri builder so it runs only after the loopback DSH Web page finishes loading; do not modify official packages or seeded profiles.

**Tech Stack:** Rust 2021, Tauri 2.11, WebView JavaScript, CSS, Cargo tests

---

### Task 1: Add the tested branding injection module

**Files:**
- Create: `src-tauri/src/branding.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Register an empty branding module and write failing unit tests**

Add `mod branding;` with the other module declarations in `src-tauri/src/lib.rs`. Create `src-tauri/src/branding.rs` with tests that reference the intended private API:

```rust
#[cfg(test)]
mod tests {
    use super::{is_dsh_web_url, BRANDING_SCRIPT};

    #[test]
    fn branding_runs_only_for_the_loopback_dsh_server() {
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
    fn branding_script_is_text_only_and_idempotent() {
        assert!(BRANDING_SCRIPT.contains("DS Studio"));
        assert!(BRANDING_SCRIPT.contains("0 0 182 24"));
        assert!(BRANDING_SCRIPT.contains("MutationObserver"));
        assert!(BRANDING_SCRIPT.contains("__DS_STUDIO_BRAND_OBSERVER__"));
        assert!(BRANDING_SCRIPT.contains("data-ds-studio-brand"));
        assert!(!BRANDING_SCRIPT.contains("data:image"));
    }
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branding::tests -- --nocapture`

Expected: compilation fails because `is_dsh_web_url` and `BRANDING_SCRIPT` do not exist.

- [ ] **Step 3: Implement URL gating, idempotent replacement, and page-load handling**

Above the tests in `src-tauri/src/branding.rs`, implement:

```rust
use tauri::{
    webview::{PageLoadEvent, PageLoadPayload},
    Runtime, Webview,
};

const BRANDING_SCRIPT: &str = r#"
(() => {
  const styleSelector = 'style[data-ds-studio-brand]';
  const svgSelector = 'button svg[viewBox="0 0 182 24"]';
  const wordmarkClass = 'ds-studio-wordmark';

  const installStyles = () => {
    if (document.querySelector(styleSelector)) return;
    const style = document.createElement('style');
    style.setAttribute('data-ds-studio-brand', 'true');
    style.textContent = `
      .${wordmarkClass} {
        display: inline-flex;
        align-items: baseline;
        gap: 0.22em;
        color: inherit;
        font-family: "Bahnschrift", "Segoe UI Variable", sans-serif;
        font-size: 23px;
        font-weight: 650;
        line-height: 1;
        letter-spacing: -0.025em;
        white-space: nowrap;
      }
      .${wordmarkClass}__ds {
        color: #3f67f3;
        background: linear-gradient(135deg, #3f67f3 0%, #1689cf 100%);
        background-clip: text;
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
      }
      @media (forced-colors: active) {
        .${wordmarkClass}__ds {
          color: CanvasText;
          background: none;
          -webkit-text-fill-color: CanvasText;
        }
      }
    `;
    document.head.append(style);
  };

  const replaceWordmark = () => {
    const svg = document.querySelector(svgSelector);
    if (!svg) return;
    const button = svg.closest('button');
    if (!button || button.querySelector(`.${wordmarkClass}`)) return;

    const wordmark = document.createElement('span');
    wordmark.className = wordmarkClass;
    wordmark.setAttribute('aria-label', 'DS Studio');

    const ds = document.createElement('span');
    ds.className = `${wordmarkClass}__ds`;
    ds.setAttribute('aria-hidden', 'true');
    ds.textContent = 'DS';

    const studio = document.createElement('span');
    studio.setAttribute('aria-hidden', 'true');
    studio.textContent = 'Studio';

    wordmark.append(ds, studio);
    svg.replaceWith(wordmark);
  };

  installStyles();
  replaceWordmark();

  if (!window.__DS_STUDIO_BRAND_OBSERVER__) {
    window.__DS_STUDIO_BRAND_OBSERVER__ = new MutationObserver(replaceWordmark);
    window.__DS_STUDIO_BRAND_OBSERVER__.observe(document.documentElement, {
      childList: true,
      subtree: true,
    });
  }
})();
"#;

fn is_dsh_web_url(url: &tauri::Url) -> bool {
    url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port().is_some()
}

pub(crate) fn handle_page_load<R: Runtime>(
    webview: &Webview<R>,
    payload: &PageLoadPayload<'_>,
) {
    if matches!(payload.event(), PageLoadEvent::Finished) && is_dsh_web_url(payload.url()) {
        if let Err(error) = webview.eval(BRANDING_SCRIPT) {
            eprintln!("failed to install DS Studio wordmark: {error}");
        }
    }
}
```

- [ ] **Step 4: Run the focused tests and verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branding::tests -- --nocapture`

Expected: both branding tests pass.

- [ ] **Step 5: Wire the page-load callback into the Tauri builder**

In `src-tauri/src/lib.rs`, insert the callback before `.setup(...)`:

```rust
        .invoke_handler(tauri::generate_handler![startup_ui_ready])
        .on_page_load(branding::handle_page_load)
        .setup(|app| {
```

- [ ] **Step 6: Run the complete Rust suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`

Expected: all existing and new tests pass.

- [ ] **Step 7: Commit the implementation**

```powershell
git add -- src-tauri/src/branding.rs src-tauri/src/lib.rs
git commit -m "feat: brand DSH sidebar as DS Studio"
```

### Task 2: Verify the packaged desktop behavior

**Files:**
- Verify only: `src-tauri/icons/icon.png`
- Verify only: `src-tauri/icons/icon.ico`

- [ ] **Step 1: Run formatting and repository checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: formatting check succeeds without changes.

Run: `git diff --check HEAD~1..HEAD`

Expected: no whitespace errors.

- [ ] **Step 2: Build the desktop application**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`

Expected: the Tauri application builds successfully.

- [ ] **Step 3: Launch and visually inspect the app**

Run: `cargo run --manifest-path src-tauri/Cargo.toml`

Verify: the expanded sidebar shows the single-line `DS Studio` wordmark; `DS` uses the blue icon-derived treatment; `Studio` follows the current theme text color; clicking the wordmark keeps its existing new-session behavior; collapsing the sidebar keeps the existing fish/toggle control; relaunching and switching the available theme do not remove the replacement.

- [ ] **Step 4: Confirm icon resources are unchanged**

Run: `git diff HEAD~1..HEAD -- src-tauri/icons`

Expected: no output.

- [ ] **Step 5: Record final repository state**

Run: `git status --short`

Expected: no uncommitted implementation changes remain.
