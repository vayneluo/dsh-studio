# Pure DSH Web Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the bundled third-party Web UI and ship a responsive, testable Windows desktop wrapper around the official DSH Web profile with a runtime under 230 MiB.

**Architecture:** A Tauri shell starts the bundled Node/DSH host without blocking the UI thread, probes the real HTTP endpoint, then navigates the WebView or renders a startup error. Build-time policy code creates and audits a Windows x64-only runtime; startup migration removes only the previously bundled `@linxin666` profile entries while preserving user data.

**Tech Stack:** Rust 2021, Tauri v2, Node.js 24, Node built-in test runner, `@deepseek-ai/dsh@0.1.0-rc.6`, NSIS.

---

## File map

- Create `scripts/runtime-policy.mjs`: testable runtime pruning and budget audit.
- Create `scripts/runtime-policy.test.mjs`: Node tests for pruning/audit behavior.
- Modify `scripts/bundle-host.mjs`: build official DSH-only runtime and call policy module.
- Modify `scripts/smoke-test.mjs`: use a temporary clean DSH home and reject third-party config.
- Create `src-tauri/src/profile.rs`: narrowly migrate the legacy bundled Web UI profile.
- Modify `src-tauri/src/sidecar.rs`: logging, hidden process, HTTP readiness, ordered tree kill.
- Modify `src-tauri/src/lib.rs`: runtime resolution and asynchronous startup orchestration.
- Modify `ui/index.html`: startup and actionable error states.
- Modify `src-tauri/tauri.conf.json`: bundle only Node and host runtime paths.
- Modify `README.md`: describe the official DSH Web desktop and build/verification commands.

## Task 1: Runtime pruning policy

**Files:**
- Create: `scripts/runtime-policy.test.mjs`
- Create: `scripts/runtime-policy.mjs`

- [ ] **Step 1: Write the failing policy tests**

Use Node's built-in test runner to create a fixture containing `keep.js`, `debug.pdb`, `code.js.map`, `types.d.ts`, `types.d.mts`, `node-pty/prebuilds/win32-x64/pty.node`, `node-pty/prebuilds/win32-arm64/pty.node`, and `node_modules/@linxin666/dsh-web-ui-all/package.json`. Assert that `pruneRuntime()` keeps the Windows x64 binary and removes debug/type/non-x64 files, then assert `auditRuntime()` rejects the forbidden package and a budget overflow.

- [ ] **Step 2: Run the test and verify RED**

Run: `node --test scripts/runtime-policy.test.mjs`

Expected: FAIL with `ERR_MODULE_NOT_FOUND` for `runtime-policy.mjs`.

- [ ] **Step 3: Implement the minimal policy module**

Export:

```js
export function pruneRuntime(hostDir) {}
export function auditRuntime(runtimeDir, { maxBytes = 230 * 1024 * 1024 } = {}) {}
```

`pruneRuntime` removes `*.pdb`, `*.map`, `*.d.ts`, `*.d.mts`, `*.d.cts`, non-`win32-x64` `node-pty/prebuilds` directories, and `node-pty/{build,deps,scripts,src,third_party,typings}`. `auditRuntime` recursively counts physical files without following reparse points, rejects any path containing `@linxin666`, and throws above the byte budget.

- [ ] **Step 4: Run the test and verify GREEN**

Run: `node --test scripts/runtime-policy.test.mjs`

Expected: all policy tests pass.

- [ ] **Step 5: Commit**

```powershell
git add scripts/runtime-policy.mjs scripts/runtime-policy.test.mjs
git commit -m "build: add runtime pruning policy"
```

## Task 2: Build and smoke an official-only DSH runtime

**Files:**
- Modify: `scripts/bundle-host.mjs`
- Modify: `scripts/smoke-test.mjs`

- [ ] **Step 1: Extend the failing audit test**

Add source assertions that `bundle-host.mjs` contains no `@linxin666`, plugin installation, pnpm invocation, or `runtime/home` seed, and that it invokes both `pruneRuntime(host)` and `auditRuntime(runtime)`.

- [ ] **Step 2: Run the test and verify RED**

Run: `node --test scripts/runtime-policy.test.mjs`

Expected: FAIL because the existing bundler installs `@linxin666/dsh-web-ui-all`.

- [ ] **Step 3: Rewrite the bundle entry**

Keep the pinned Node and DSH installation, delete a stale `runtime/home`, call the pruning policy, run `dsh web --dump-default-config` with an isolated temporary home, and audit the result. Do not install pnpm or any plugin.

- [ ] **Step 4: Make smoke testing isolated and exact**

Create a temporary DSH home with `mkdtemp`, launch the bundled sidecar with `--port 0`, wait for the printed URL, fetch `/`, assert HTTP 200-399, run `--dump-config`, reject `@linxin666`, kill the process tree, and delete only the temporary home in `finally`.

- [ ] **Step 5: Rebuild and verify**

Run:

```powershell
node scripts/bundle-host.mjs
node --test scripts/runtime-policy.test.mjs
node scripts/smoke-test.mjs
```

Expected: policy tests pass, smoke prints `SMOKE OK`, runtime audit reports at most 230 MiB and no forbidden package.

- [ ] **Step 6: Commit**

```powershell
git add scripts/bundle-host.mjs scripts/smoke-test.mjs
git commit -m "build: remove bundled web ui integration"
```

## Task 3: Migrate the legacy Web profile safely

**Files:**
- Create: `src-tauri/src/profile.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing manifest tests**

Cover these fixtures:

1. Old bundled manifest with only `@linxin666/dsh-web-ui-all` becomes the official two-bundle profile with no dependencies.
2. A manifest with a user plugin keeps that dependency and bundle while removing both known `@linxin666` packages.
3. An official-only manifest is unchanged.
4. Invalid JSON returns an error without overwriting the source file.

- [ ] **Step 2: Run and verify RED**

Run: `cargo test profile::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because module/function does not exist.

- [ ] **Step 3: Implement migration**

Add:

```rust
pub fn migrate_legacy_web_profile(dsh_home: &Path) -> Result<Migration, String>
```

Read `profiles/web/package.json`, remove only `@linxin666/dsh-web-ui-all` and `@linxin666/dsh-skins` from dependencies and bundle order, write through a sibling temporary file followed by rename, and return whether the old profile contained only bundled integration dependencies. When it did, delete `profiles/web/node_modules`, its lockfile and `.npmrc`; leave all other DSH home data intact.

- [ ] **Step 4: Run and verify GREEN**

Run: `cargo test profile::tests --manifest-path src-tauri/Cargo.toml`

Expected: all migration tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/profile.rs src-tauri/src/lib.rs
git commit -m "fix: migrate legacy enhanced web profile"
```

## Task 4: Make sidecar readiness and shutdown reliable

**Files:**
- Modify: `src-tauri/src/sidecar.rs`

- [ ] **Step 1: Replace TCP tests with failing HTTP tests**

The positive fixture must accept a connection and return `HTTP/1.1 200 OK`; a listener returning `503` must remain unready until timeout. Add an assertion that the generated process command uses a log file and the Windows `CREATE_NO_WINDOW` flag through a small command-construction helper.

- [ ] **Step 2: Run and verify RED**

Run: `cargo test sidecar::tests --manifest-path src-tauri/Cargo.toml`

Expected: the current TCP-only probe incorrectly treats 503/listening as ready.

- [ ] **Step 3: Implement HTTP probing and logs**

Change `spawn` to accept `&Path` values plus a log path, open/clone the log file for stdout and stderr, apply `CREATE_NO_WINDOW`, and return contextual IO errors. Implement a raw loopback HTTP GET probe that accepts 2xx/3xx only and has short per-attempt read/write timeouts.

- [ ] **Step 4: Order tree termination**

Run hidden `taskkill /F /T /PID <pid>` with `.status()` and only then call `child.kill()`/`wait()` as fallback. This prevents killing the root before `taskkill` enumerates descendants.

- [ ] **Step 5: Run and verify GREEN**

Run: `cargo test sidecar::tests --manifest-path src-tauri/Cargo.toml`

Expected: all sidecar tests pass.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/sidecar.rs
git commit -m "fix: harden sidecar lifecycle"
```

## Task 5: Make Tauri startup asynchronous and development-aware

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/index.html`

- [ ] **Step 1: Write failing runtime path tests**

Extract a pure resolver that prefers `<resource_dir>/runtime` and, only under a supplied development flag, falls back to `<manifest_parent>/runtime`. Assert missing production resources return a descriptive error.

- [ ] **Step 2: Run and verify RED**

Run: `cargo test app_tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because no resolver exists.

- [ ] **Step 3: Implement non-blocking orchestration**

In `setup`, resolve paths, create the DSH home/log directory, run profile migration, spawn the sidecar, store it in managed state, and return immediately. A background thread calls HTTP readiness for up to 90 seconds; success schedules WebView navigation, failure evaluates `window.showStartupError(message, logPath)` and terminates the sidecar.

- [ ] **Step 4: Implement startup/error UI**

Add accessible status markup, an error panel, selectable log path, and:

```js
window.showStartupError = (message, logPath) => { /* switch state and render textContent */ }
```

Use `textContent` only; never inject error strings as HTML.

- [ ] **Step 5: Run tests and compile**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass and the app compiles.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/lib.rs ui/index.html
git commit -m "fix: make desktop startup responsive"
```

## Task 6: Bundle only the official runtime and document it

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `README.md`

- [ ] **Step 1: Add failing config assertions**

Extend the Node policy test to parse `tauri.conf.json` and assert resource mappings include only `../runtime/node/node.exe` and `../runtime/host`, never the whole `../runtime` directory.

- [ ] **Step 2: Run and verify RED**

Run: `node --test scripts/runtime-policy.test.mjs`

Expected: FAIL because the current config bundles `../runtime`, including stale `runtime/home`.

- [ ] **Step 3: Narrow resources and update README**

Map only Node and host resources. Remove enhanced UI claims and document `node scripts/bundle-host.mjs`, `node scripts/smoke-test.mjs`, `cargo tauri dev`, and `cargo tauri build`.

- [ ] **Step 4: Run source/config checks**

Run:

```powershell
node --test scripts/runtime-policy.test.mjs
rg -n "@linxin666|dsh-web-ui|dsh-skins" README.md scripts src-tauri ui
```

Expected: tests pass and ripgrep returns no matches outside explicit migration constants/tests.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/tauri.conf.json README.md scripts/runtime-policy.test.mjs
git commit -m "docs: describe official dsh web desktop"
```

## Task 7: Full verification and interactive handoff

**Files:**
- No tracked source changes expected.

- [ ] **Step 1: Run the full fast gate**

```powershell
node --test scripts/runtime-policy.test.mjs
node scripts/smoke-test.mjs
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 2: Build the installer and measure artifacts**

Run `cargo tauri build --manifest-path src-tauri/Cargo.toml`.

Expected: NSIS build exits 0, runtime is at most 230 MiB, installer at most 60 MiB, and the build no longer spends minutes traversing the removed profile tree.

- [ ] **Step 3: Launch development desktop**

Run `cargo tauri dev --manifest-path src-tauri/Cargo.toml` in a persistent terminal session. Wait for the official DSH Web page, confirm the page has no task-board/SSH/whale controls and no console errors, then keep the application and dev process running for user inspection.

- [ ] **Step 4: Report exact evidence**

Report test counts, smoke output, runtime/installer sizes, build time, live process/session state, and any remaining limitations. Do not claim completion without fresh output from every gate above.
