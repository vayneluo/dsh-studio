# DS Studio Sidebar Wordmark Design

## Goal

Replace the official DeepSeek wordmark in the expanded DSH Web sidebar with a more prominent, desktop-specific `DS Studio` text wordmark. Keep the existing desktop application icon and the collapsed-sidebar control unchanged.

## Visual Design

- Render a single-line, text-only `DS Studio` wordmark.
- Render `DS` in a blue treatment derived from the desktop icon and `Studio` in the current theme's primary text color.
- Use an approximately 22 px, semibold display with enough contrast to read clearly in both light and dark themes.
- Do not add a badge, border, background plate, or separate graphic mark.
- Preserve the official collapsed-sidebar fish/toggle control because it communicates the existing collapse interaction within the 56 px rail.

## Integration

The Tauri desktop shell will own this desktop-only branding adjustment. A small initialization script will run when the official DSH page loads and will:

1. Locate the official full wordmark SVG by its stable wordmark view box within the sidebar brand button.
2. Replace only the SVG content with an accessible `DS Studio` text node structure.
3. Preserve the containing button, including its existing click behavior, focus behavior, and layout role.
4. Install scoped styles identified by a `data-ds-studio-brand` marker.
5. Observe document mutations so the replacement is restored if React remounts the sidebar.

The script must be idempotent. Repeated page-load callbacks or mutation notifications must not duplicate styles, observers, or wordmark nodes.

This approach intentionally avoids editing installed `node_modules` or changing a user's seeded DSH profile. It therefore applies to both fresh and existing installations and is not lost when the bundled DSH runtime is rebuilt.

## Accessibility and Compatibility

- The visible text must be represented as text, not a new image.
- The existing brand button remains keyboard accessible and retains its original action.
- The wordmark inherits the theme's primary label color for `Studio`; `DS` uses a blue that meets readable contrast on the supported sidebar surfaces.
- The replacement does nothing when the official wordmark is absent or the sidebar is collapsed.
- Reduced-motion behavior is unaffected because the replacement adds no animation.

## Verification

- Add unit coverage for the generated branding script and its page-load integration.
- Run the Rust test suite to confirm startup, navigation, and shutdown behavior remains intact.
- Launch the desktop app and visually verify the expanded sidebar in both available themes, the collapsed sidebar, wordmark click behavior, and the unchanged Windows icon.

## Out of Scope

- Redesigning or regenerating the desktop icon.
- Rebranding the startup screen, window title, installer metadata, or other DSH surfaces.
- Replacing the collapsed-sidebar fish/toggle control.
- Forking or patching official DeepSeek packages.
