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

  const replaceHeadline = (headline) => {
    if (!(headline instanceof HTMLSpanElement) || !headline.parentElement) return;
    const source = headline.textContent?.trim();
    const replacement = replacements.get(source);
    if (!replacement) return;
    const siblingLabels = [...headline.parentElement.children]
      .map((element) => element.textContent?.trim());
    if (!siblingLabels.some((label) => previewLabels.has(label))) return;
    headline.textContent = replacement;
  };

  const applyWithin = (root) => {
    if (!(root instanceof Element)) return;
    if (root.matches('span')) replaceHeadline(root);
    for (const headline of root.querySelectorAll('span')) replaceHeadline(headline);
  };

  applyWithin(document.documentElement);
  if (!window.__DS_STUDIO_COPY_OBSERVER__) {
    window.__DS_STUDIO_COPY_OBSERVER__ = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') {
          replaceHeadline(mutation.target.parentElement);
          continue;
        }
        for (const added of mutation.addedNodes) {
          applyWithin(added instanceof Element ? added : added.parentElement);
        }
      }
    });
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
            assert!(
                INTERFACE_COPY_SCRIPT.contains(expected),
                "missing {expected}"
            );
        }
        assert!(!INTERFACE_COPY_SCRIPT.contains("welcomeTitle"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("BrandWordmark"));
    }
}
