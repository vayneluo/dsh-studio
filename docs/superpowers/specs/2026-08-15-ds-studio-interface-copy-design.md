# DS Studio Interface Copy Design

## Goal

Unify the two user-identified product labels with the `DS Studio` name while leaving the existing iconography and the rest of the official DSH interface unchanged.

## Confirmed Copy

- Windows title bar: `DS Studio`
- Chinese home hero headline: `与 DS Studio 一起，探索未至之境`
- English home hero headline: `Explore the unknown with DS Studio`

The home hero's fish graphic and `预览版` / `Preview` badge remain unchanged.

## Integration

The Windows title bar is owned by the Tauri window configuration. Change only the main window's `title`; do not rename the executable, package identifier, product metadata, publisher, or icon resources.

The home headline is owned by the official DSH Web client. The desktop shell will apply a small, idempotent page-load script that replaces only the exact official hero strings (`探索未至之境` and `Into the Unknown`) after the loopback DSH page loads. A mutation observer will reapply the copy when React remounts the hero or the user switches locale.

The script must not patch `node_modules`, seeded profiles, the sidebar wordmark, onboarding dialogs, or general conversation copy.

## Accessibility and Compatibility

- Replace text content in the existing headline element so its semantic and layout role remains intact.
- Preserve the fish SVG, preview badge, input controls, and all click/focus behavior.
- Support both official GUI locales.
- Run only on the loopback HTTP DSH page after loading finishes; do nothing on the Tauri startup page or unrelated URLs.
- Repeated callbacks and mutations must not duplicate text or observers.

## Verification

- Unit-test the title configuration and the page script's URL gate, bilingual source strings, replacement strings, and idempotency marker.
- Run the complete Rust and repository test suites.
- Build and launch the desktop app to verify the two screenshot locations, locale switching, unchanged fish/preview elements, and unchanged icon resources.

## Out of Scope

- Redesigning or regenerating the desktop icon.
- Replacing the sidebar's official wordmark.
- Editing welcome, API-key, settings, or model-provider copy.
- Changing `productName`, the bundle identifier, executable name, installer metadata, or publisher.
- Forking or patching official DeepSeek packages.
