use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::OnceLock;
use tauri::{
    webview::{PageLoadEvent, PageLoadPayload},
    Runtime, Webview,
};

const BRAND_PNG: &[u8] = include_bytes!("../../ui/brand.png");
static RENDERED_INTERFACE_COPY_SCRIPT: OnceLock<String> = OnceLock::new();

const INTERFACE_COPY_SCRIPT: &str = r#"
(() => {
  const brandDataUrl = '__DS_STUDIO_BRAND_DATA_URL__';
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
    [
      '配置 DeepSeek 官方模型，即可开始使用。',
      '为 DS Studio 配置 DeepSeek 官方模型，即可开始使用。',
    ],
    [
      'Configure the official DeepSeek provider to start building.',
      'Configure the official DeepSeek provider for DS Studio to get started.',
    ],
  ]);

  const replaceWordmarks = (root) => {
    const selector = 'svg[viewBox="0 0 182 24"][aria-hidden="true"]';
    const wordmarks = root.matches(selector)
      ? [root]
      : [...root.querySelectorAll(selector)];
    for (const wordmark of wordmarks) {
      if (wordmark.parentElement?.tagName !== 'BUTTON') continue;
      const brand = document.createElement('img');
      brand.setAttribute('data-ds-studio-wordmark', '');
      brand.src = brandDataUrl;
      brand.alt = 'DS Studio';
      brand.decoding = 'async';
      brand.draggable = false;
      Object.assign(brand.style, {
        display: 'block',
        width: '182px',
        height: '24px',
        objectFit: 'cover',
        objectPosition: '50% 50%',
        borderRadius: '5px',
        backgroundColor: '#fff',
      });
      wordmark.replaceWith(brand);
    }
  };

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

  const replaceProductCopy = (element) => {
    if (!(element instanceof HTMLElement) || element.childElementCount !== 0) return;
    if (!element.closest('[role="dialog"]')) return;
    const replacement = copyReplacements.get(normalize(element.textContent));
    if (replacement && element.textContent !== replacement) {
      element.textContent = replacement;
    }
  };

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

  applyWithin(document.documentElement);
  if (!window.__DS_STUDIO_COPY_OBSERVER__) {
    window.__DS_STUDIO_COPY_OBSERVER__ = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') {
          applyWithin(mutation.target.parentElement);
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

fn rendered_interface_copy_script() -> &'static str {
    RENDERED_INTERFACE_COPY_SCRIPT.get_or_init(|| {
        let brand_data_url = format!("data:image/png;base64,{}", STANDARD.encode(BRAND_PNG));
        INTERFACE_COPY_SCRIPT.replace("__DS_STUDIO_BRAND_DATA_URL__", &brand_data_url)
    })
}

fn is_dsh_web_url(url: &tauri::Url, expected_port: Option<u16>) -> bool {
    expected_port.is_some_and(|port| {
        url.scheme() == "http" && url.host_str() == Some("127.0.0.1") && url.port() == Some(port)
    })
}

pub(crate) fn handle_page_load<R: Runtime>(
    webview: &Webview<R>,
    payload: &PageLoadPayload<'_>,
    expected_port: Option<u16>,
) {
    if matches!(payload.event(), PageLoadEvent::Finished)
        && is_dsh_web_url(payload.url(), expected_port)
    {
        if let Err(error) = webview.eval(rendered_interface_copy_script()) {
            eprintln!("failed to install DS Studio interface copy: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_dsh_web_url, rendered_interface_copy_script, INTERFACE_COPY_SCRIPT};

    #[test]
    fn copy_runs_only_for_the_loopback_dsh_server() {
        assert!(is_dsh_web_url(
            &tauri::Url::parse("http://127.0.0.1:43123/").unwrap(),
            Some(43123),
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("http://127.0.0.1:43124/").unwrap(),
            Some(43123),
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("http://127.0.0.1:43123/").unwrap(),
            None,
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("http://127.0.0.1/").unwrap(),
            None,
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("tauri://localhost/").unwrap(),
            Some(43123),
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("https://127.0.0.1:43123/").unwrap(),
            Some(43123),
        ));
        assert!(!is_dsh_web_url(
            &tauri::Url::parse("http://localhost:43123/").unwrap(),
            Some(43123),
        ));
    }

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
    fn script_embeds_the_supplied_brand_image_instead_of_rendering_plain_text() {
        let source = include_str!("interface_copy.rs");

        for expected in [
            "include_bytes!(\"../../ui/brand.png\")",
            "data:image/png;base64",
            "createElement('img')",
            "alt = 'DS Studio'",
            "objectFit: 'cover'",
        ] {
            assert!(source.contains(expected), "missing {expected}");
        }

        assert!(!INTERFACE_COPY_SCRIPT.contains("label.textContent = 'DS Studio'"));
    }

    #[test]
    fn rendered_script_contains_the_png_data_url_and_no_template_marker() {
        let script = rendered_interface_copy_script();

        assert!(script.contains("data:image/png;base64,iVBOR"));
        assert!(!script.contains("__DS_STUDIO_BRAND_DATA_URL__"));
    }

    #[test]
    fn script_removes_only_the_hero_preview_badge_and_preserves_internal_names() {
        assert!(INTERFACE_COPY_SCRIPT.contains("badge.remove()"));
        assert!(INTERFACE_COPY_SCRIPT.contains("data-ds-studio-hero"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("querySelectorAll('*')"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("replaceAll('DeepSeek'"));
        assert!(!INTERFACE_COPY_SCRIPT.contains("replaceAll('DSH'"));
    }

    #[test]
    fn script_maps_each_welcome_paragraph_as_rendered() {
        for paragraph_entry in [
            "持续演化。',",
            "加入 DSH 插件生态。',",
            "over the coming months.\",",
            "join the DSH plugin ecosystem.',",
        ] {
            assert!(
                INTERFACE_COPY_SCRIPT.contains(paragraph_entry),
                "welcome paragraph is not mapped separately: {paragraph_entry}"
            );
        }
    }

    #[test]
    fn script_limits_product_copy_to_the_onboarding_dialog() {
        assert!(INTERFACE_COPY_SCRIPT.contains("element.closest('[role=\"dialog\"]')"));
    }

    #[test]
    fn script_reapplies_all_copy_for_in_place_locale_changes() {
        assert!(INTERFACE_COPY_SCRIPT
            .contains("applyWithin(mutation.target.parentElement);\n          continue;"));
    }
}
