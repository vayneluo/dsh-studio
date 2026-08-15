# DS Studio Interface Copy Design

## Goal

Present one consistent `DS Studio` product identity across the Windows title bar, expanded sidebar, home hero, and first-run onboarding while preserving provider names, application behavior, and the existing desktop icon.

## Confirmed Copy

- Windows title bar: `DS Studio`
- Expanded sidebar wordmark: plain text `DS Studio`
- Chinese home hero headline: `与 DS Studio 一起，探索未至之境`
- English home hero headline: `Explore the unknown with DS Studio`
- Home hero badge: remove `预览版` / `Preview`
- Chinese welcome title: `欢迎使用 DS Studio`
- English welcome title: `Welcome to DS Studio`
- Chinese welcome body: `DS Studio 目前处于预览阶段，产品体验和基础能力会持续改进，欢迎反馈建议。\n\n我们期待与你一起，在开放、可复用、可组合的基础设施之上，共同探索智能上限。`
- English welcome body: `DS Studio is currently in preview. The product experience and foundational capabilities will continue to improve, and we welcome your feedback.\n\nWe look forward to exploring the limits of intelligence with you on open, reusable, and composable infrastructure.`
- Chinese API-key description: `为 DS Studio 配置 DeepSeek 官方模型，即可开始使用。`
- English API-key description: `Configure the official DeepSeek provider for DS Studio to get started.`

The first-run welcome body uses concise DS Studio product copy instead of describing the product as DeepSeek Harness or addressing Harness developers. The separate API-key step continues to name DeepSeek because it refers to the DeepSeek model provider, but its description identifies DS Studio as the application being configured.

## Integration

The Windows title bar is owned by the Tauri window configuration. Change only the main window's `title`; do not rename the executable, package identifier, product metadata, publisher, or icon resources.

The visible Web UI labels are owned by official DSH client packages. The desktop shell will apply one small, idempotent page-load script after the exact loopback origin selected for the running DSH sidecar loads. It will:

- replace the expanded sidebar's `BrandWordmark` presentation with a styled text node while preserving the existing new-session button and its accessible behavior;
- replace only the exact official hero strings (`探索未至之境` and `Into the Unknown`) and remove only their sibling preview badge;
- replace the exact bilingual first-run welcome title and body strings with DS Studio copy;
- update the exact API-key onboarding description so DeepSeek remains clearly identified as a provider rather than the product name.

A mutation observer will reapply these changes when React remounts a target or the user switches locale.

The script must not patch `node_modules`, seeded profiles, settings data, model-provider identifiers, technical runtime names, or general conversation copy. It must never perform a broad search-and-replace for `DeepSeek`, `DSH`, or `Harness` because those terms remain valid for upstream packages, provider configuration, and plugin/runtime concepts.

## Accessibility and Compatibility

- Preserve the sidebar brand button's click, focus, and accessible-name behavior while replacing its visible official wordmark with plain text.
- Replace text content in existing hero and onboarding elements so their semantic and layout roles remain intact.
- Preserve the home fish SVG, all input controls, and all click/focus behavior; remove only the hero preview badge.
- Support both official GUI locales.
- Run only on the loopback HTTP origin whose port matches the running DSH sidecar after loading finishes; do nothing before that port is published, on the Tauri startup page, or on unrelated URLs and loopback ports.
- Repeated callbacks and mutations must not duplicate text or observers.

## Verification

- Unit-test the title configuration and the page script's URL gate, exact sidebar/hero/onboarding targets, bilingual source and replacement strings, preview removal, and idempotency marker.
- Run the complete Rust and repository test suites.
- Build and launch the desktop app with isolated first-run data to verify the title bar, pure-text expanded sidebar, first-run dialogs, hero copy, absent preview badge, locale switching, preserved interactions, and unchanged desktop icon resources.

## Out of Scope

- Redesigning or regenerating the desktop icon.
- Replacing the collapsed sidebar control's compact fish icon.
- Renaming the DeepSeek model provider, DSH plugin ecosystem, packages, executable, profile, or runtime internals.
- Editing unrelated settings, model-provider, or conversation copy.
- Changing `productName`, the bundle identifier, executable name, installer metadata, or publisher.
- Forking or patching official DeepSeek packages.
