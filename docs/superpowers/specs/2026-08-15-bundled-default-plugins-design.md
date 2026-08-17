# Bundled Default Plugins Design

**Date:** 2026-08-15
**Status:** Approved for implementation

## Goal

Ship four curated DSH Web plugins inside the Windows installer so a genuinely new installation has them enabled on its first launch without network access or a restart. Existing `profiles/web` data must never be seeded, merged, or overwritten by this feature.

## Fixed plugin set

| Display name | Reproducible install target | Expected package version |
| --- | --- | --- |
| dsh-at-file | `github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850` | 0.6.0 |
| dsh-better-sidebar | `dsh-better-sidebar@0.12.1` | 0.12.1 |
| dshmarket | `dshmarket@1.2.2` | 1.2.2 |
| dsh-message-edit | `dsh-message-edit@0.2.1` | 0.2.1 |

All four packages declare MIT licenses. Their distributed license files remain in the seed, and the repository documents the bundled package names and versions.

## Selected approach

The build creates a standalone default-profile seed with DSH's own plugin command. Tauri packages that seed as a resource alongside, but separate from, the official Node/DSH host. At runtime the desktop app copies the seed only when the entire destination `profiles/web` path is absent.

This keeps the official host immutable, follows DSH's profile format, avoids first-launch networking, and gives the desktop app a narrow ownership boundary: it owns creation of a missing profile but never owns an existing one.

Alternatives rejected:

- Installing the plugins directly in the host `node_modules` couples plugin discovery to undocumented module-resolution behavior.
- Shipping a custom compressed archive adds first-launch extraction and recovery logic without materially reducing the NSIS payload.
- First-launch online installation keeps the installer smaller but introduces network failure and restart behavior that the bundled seed avoids.

## Build architecture

`scripts/bundle-host.mjs` continues to build a clean staging runtime and adds these steps before publishing it:

1. Create an isolated temporary DSH home inside staging.
2. Sanitize all inherited `npm_config_*` values and force `https://registry.npmjs.org/` for the default registry and the `@deepseek-ai` scope.
3. Set pnpm peer auto-installation off. DSH `0.1.0-rc.6` otherwise attempts to resolve stable peer ranges that do not exist even though the host already supplies those peers.
4. Invoke the downloaded Node binary and bundled DSH/pnpm implementation; no global npm or pnpm is required.
5. Install the five fixed targets at the web workspace root.
6. Convert the resulting profile into a portable physical tree. The packaged seed must contain no symbolic links or junctions into a build-machine pnpm store.
7. Apply the existing Windows x64 pruning policy to the seed: remove PDB files, source maps, type declarations, development sources, and non-Windows-x64 native artifacts while preserving runtime code and licenses.
8. Validate the installed package versions and the profile bundle order.
9. Run DSH Web from the staged profile and require a successful HTTP smoke response.
10. Audit and atomically publish the complete staging runtime.

The packaged resource layout is:

```text
runtime/
  node/node.exe
  host/...
  profile-seed/
    profiles/web/...
```

The web profile must contain exactly the two official bundles followed by the five fixed plugin bundles. No `@linxin666` package may be present anywhere in the runtime or seed.

## Runtime seeding

Startup retains the current legacy-profile migration and then performs new-profile seeding before the sidecar starts:

1. Resolve the bundled `runtime/profile-seed/profiles/web` directory and the app-data `profiles/web` destination.
2. If the destination path exists in any form, return `SkippedExisting` without reading or changing it. This includes upgrades and reinstalls that retain app data.
3. Validate the bundled seed manifest before copying.
4. Copy the seed to one exact app-owned sibling staging directory on the same volume.
5. Validate the copied manifest and reject reparse points.
6. Atomically rename the staging directory to `profiles/web`.
7. Start DSH Host only after the rename succeeds, so all five plugins are active on the first displayed Web session.

The seeding function has a small result contract (`Seeded` or `SkippedExisting`) and no package-manager responsibilities. Existing profile migration remains the current narrowly scoped removal of the legacy `@linxin666` integration; adding the five new defaults to existing profiles is explicitly out of scope.

## Failure handling

- Package download, version mismatch, non-portable links, bundle mismatch, HTTP smoke failure, or size-budget failure stops the build.
- Runtime validation or copy failure removes only the exact app-owned staging path, leaves `profiles/web` absent, and uses the existing startup error UI and log path. A later launch may retry.
- The final destination is never exposed partially. No fallback writes an official-only profile over a failed seed.
- Existing profiles are not repaired or modified by seeding, even if their manifest is missing or invalid.

## Size policy

The measured five-plugin profile is 92.46 MiB before pruning and approximately 25.24 MiB after the existing Windows x64 pruning rules. Calibrating ordinary ZIP output against the current 45.00 MiB NSIS installer gives an expected final installer size near 62.5 MiB.

Hard gates:

- Complete expanded packaged runtime, including the profile seed: at most 260 MiB.
- Portable profile seed: at most 35 MiB.
- Final NSIS installer: at most 70 MiB.

## Verification

Node tests cover fixed package targets, registry/environment isolation, pruning, link rejection, exact versions, exact seven-bundle order, forbidden packages, and byte budgets.

Rust tests cover:

- seeding when `profiles/web` is absent;
- byte-for-byte preservation when any destination already exists;
- rejection of an invalid bundled or copied manifest;
- cleanup of the owned staging path after copy failure;
- absence of a visible partial destination;
- continued behavior of legacy profile migration.

End-to-end verification runs:

- Node policy tests;
- Rust unit tests;
- `cargo fmt --check`;
- Clippy with warnings denied;
- DSH Web HTTP smoke against the staged seed;
- release/NSIS build;
- final runtime, installer-size, asset hash, and clean-worktree checks.

## Non-goals

- Automatically adding the plugins to existing profiles.
- Updating the five plugins at application startup.
- Providing a custom plugin-management UI beyond the bundled `dshmarket` plugin.
- Reintroducing the removed `@linxin666` enhanced Web UI or skins.
