# Bundled Default Plugins Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bundle five fixed DSH Web plugins in a portable profile seed that is atomically installed only for users who do not already have `profiles/web`.

**Architecture:** A focused Node build module creates, flattens, prunes, validates, and smokes a default DSH profile inside the clean runtime staging tree. A focused Rust module validates and atomically copies that seed into app data only when the destination profile path is absent; the existing migration and sidecar modules retain their current responsibilities.

**Tech Stack:** Node.js 24 ESM, DSH `0.1.0-rc.6`, downloaded npm CLI, Rust 2021, Tauri 2, `serde_json`, Windows filesystem APIs, Node test runner, Cargo test/Clippy, NSIS.

---

## File structure

- Create `scripts/default-profile.mjs`: fixed plugin catalog, portable profile construction, manifest/version validation, and seed audit.
- Create `scripts/default-profile.test.mjs`: pure tests for plugin catalog, manifest validation, size/link policy, and injected build commands.
- Modify `scripts/runtime-policy.mjs`: total runtime and seed byte-budget constants plus reusable physical-tree/reparse auditing.
- Modify `scripts/runtime-policy.test.mjs`: total/seed budgets and Tauri resource assertions.
- Modify `scripts/bundle-host.mjs`: call the default-profile builder before pruning/auditing/publishing staging.
- Modify `scripts/smoke-test.mjs`: smoke either a fresh official profile or the bundled seed, with exact bundle assertions.
- Create `src-tauri/src/profile_seed.rs`: validate, copy, clean up, and atomically publish only a missing profile.
- Create `src-tauri/src/profile_seed/tests.rs`: fresh/existing/invalid/copy-failure seeding tests.
- Modify `src-tauri/src/lib.rs`: seed after legacy migration and before spawning the sidecar.
- Modify `src-tauri/tauri.conf.json`: bundle `runtime/profile-seed` and bump the desktop version.
- Modify `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`: bump the package version to `0.1.1`.
- Modify `README.md`: document the five defaults, new size gates, and fresh-install-only behavior.

### Task 1: Define and test the default-profile contract

**Files:**
- Create: `scripts/default-profile.mjs`
- Create: `scripts/default-profile.test.mjs`
- Modify: `scripts/runtime-policy.mjs`
- Modify: `scripts/runtime-policy.test.mjs`

- [ ] **Step 1: Write failing catalog and validation tests**

Create tests that require the exact targets, versions, official bundles, no extra dependencies, no `@linxin666`, no reparse points, and a 35 MiB seed budget:

```js
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import {
  DEFAULT_BUNDLES,
  DEFAULT_PLUGINS,
  validateDefaultProfile,
} from './default-profile.mjs'

test('default plugin catalog is fixed and reproducible', () => {
  assert.deepEqual(DEFAULT_PLUGINS.map(({ target }) => target), [
    'github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850',
    '@liustack/modlens@3.16.6',
    'dsh-better-sidebar@0.12.1',
    'dshmarket@1.2.2',
    'dsh-message-edit@0.2.1',
  ])
  assert.deepEqual(DEFAULT_BUNDLES.slice(0, 2), [
    '@deepseek-ai/dsh-base',
    '@deepseek-ai/dsh-web-app',
  ])
})

test('validation rejects an extra profile dependency', () => {
  const fixture = makeValidProfileFixture()
  const manifest = JSON.parse(readFileSync(join(fixture, 'package.json'), 'utf8'))
  manifest.dependencies['unexpected-plugin'] = '1.0.0'
  writeFileSync(join(fixture, 'package.json'), JSON.stringify(manifest))
  assert.throws(() => validateDefaultProfile(fixture), /dependencies/i)
})
```

- [ ] **Step 2: Run the focused Node tests and verify RED**

Run: `node --test scripts/default-profile.test.mjs scripts/runtime-policy.test.mjs`

Expected: FAIL because `scripts/default-profile.mjs`, seed budgets, and reparse auditing do not exist.

- [ ] **Step 3: Implement the pure contract and audits**

Define immutable catalog data and validation with this public surface:

```js
export const DEFAULT_PROFILE_BUDGET_BYTES = 35 * 1024 * 1024
export const DEFAULT_RUNTIME_BUDGET_BYTES = 260 * 1024 * 1024
export const DEFAULT_INSTALLER_BUDGET_BYTES = 70 * 1024 * 1024

export const DEFAULT_PLUGINS = Object.freeze([
  { bundle: 'dsh-at-file', packageName: 'dsh-at-file', version: '0.6.0', target: 'github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850' },
  { bundle: '@liustack/modlens', packageName: '@liustack/modlens', version: '3.16.6', target: '@liustack/modlens@3.16.6' },
  { bundle: 'dsh-better-sidebar', packageName: 'dsh-better-sidebar', version: '0.12.1', target: 'dsh-better-sidebar@0.12.1' },
  { bundle: 'dshmarket', packageName: 'dshmarket', version: '1.2.2', target: 'dshmarket@1.2.2' },
  { bundle: 'dsh-message-edit', packageName: 'dsh-message-edit', version: '0.2.1', target: 'dsh-message-edit@0.2.1' },
])

export const DEFAULT_BUNDLES = Object.freeze([
  '@deepseek-ai/dsh-base',
  '@deepseek-ai/dsh-web-app',
  ...DEFAULT_PLUGINS.map(({ bundle }) => bundle),
])

export const validateDefaultProfile = (profileDir) => {
  const manifest = JSON.parse(readFileSync(join(profileDir, 'package.json'), 'utf8'))
  assert.deepEqual(manifest.dependencies, Object.fromEntries(
    DEFAULT_PLUGINS.map(({ packageName, target }) => [packageName, dependencySpec(target)]),
  ))
  assert.deepEqual(manifest.dsh?.profile?.bundles, DEFAULT_BUNDLES)
  for (const plugin of DEFAULT_PLUGINS) {
    const installed = JSON.parse(readFileSync(
      join(profileDir, 'node_modules', ...plugin.packageName.split('/'), 'package.json'),
      'utf8',
    ))
    if (installed.version !== plugin.version) throw new Error(`version mismatch for ${plugin.packageName}`)
  }
  auditPortableTree(profileDir, { maxBytes: DEFAULT_PROFILE_BUDGET_BYTES })
}
```

Use explicit errors instead of Node assertion errors in production code, and make the test fixture use the exported catalog so it remains readable.

Also export `auditInstaller(path, { maxBytes = DEFAULT_INSTALLER_BUDGET_BYTES } = {})`, implemented with `statSync(path).size`, so the 70 MiB release limit is an executable gate rather than a hand-calculated check.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run: `node --test scripts/default-profile.test.mjs scripts/runtime-policy.test.mjs`

Expected: all focused tests PASS.

- [ ] **Step 5: Commit the contract**

```powershell
git add scripts/default-profile.mjs scripts/default-profile.test.mjs scripts/runtime-policy.mjs scripts/runtime-policy.test.mjs
git commit -m "build: define bundled default profile"
```

### Task 2: Build a portable seeded profile

**Files:**
- Modify: `scripts/default-profile.mjs`
- Modify: `scripts/default-profile.test.mjs`
- Modify: `scripts/bundle-host.mjs`

- [ ] **Step 1: Write failing command-isolation and portability tests**

Inject a command runner and assert that the builder:

```js
test('builder uses downloaded tools and isolated npm settings', () => {
  const calls = []
  buildDefaultProfile({
    node: 'staged-node.exe',
    npmCli: 'staged-npm-cli.js',
    dshBin: 'staged-dsh.js',
    staging: fixture,
    run: (command, args, options) => calls.push({ command, args, env: options.env }),
  })

  assert.equal(calls[0].command, 'staged-node.exe')
  assert.match(calls[0].args.join(' '), /plugin --profile web add -w/)
  assert.equal(calls[0].env.NPM_CONFIG_REGISTRY, 'https://registry.npmjs.org/')
  assert.equal(calls[0].env.NPM_CONFIG_AUTO_INSTALL_PEERS, 'false')
  assert.equal(Object.keys(calls[0].env).some((key) => key.toLowerCase() === 'npm_config_registry' && calls[0].env[key].includes('haizol')), false)
})
```

Add a filesystem test proving the final seed contains no `.pnpm`, `.bin`, symlink, or junction.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `node --test scripts/default-profile.test.mjs`

Expected: FAIL because `buildDefaultProfile` is not implemented.

- [x] **Step 3: Implement staged DSH install as a hoisted physical tree**

Implement this sequence in `buildDefaultProfile`:

```js
export const buildDefaultProfile = ({ node, dshBin, staging, run = runCommand }) => {
  const seedHome = join(staging, 'profile-seed')
  const profileDir = join(seedHome, 'profiles', 'web')
  const isolated = createIsolatedNpmEnvironment(staging, node)

  run(node, [
    dshBin, 'plugin', '--profile', 'web', 'add', '-w',
    '--config.node-linker=hoisted', '--config.auto-install-peers=false',
    immutableTarballForGitPlugin, ...DEFAULT_PLUGINS.slice(1).map(({ target }) => target),
  ], { env: { ...isolated, DSH_HOME: seedHome } })

  normalizeProfileManifest(profileDir)
  rmSync(join(profileDir, 'node_modules', '.pnpm'), { recursive: true, force: true })
  rmSync(join(profileDir, 'node_modules', '.bin'), { recursive: true, force: true })
  rmSync(join(profileDir, 'node_modules', '.modules.yaml'), { force: true })
  rmSync(join(profileDir, 'pnpm-lock.yaml'), { force: true })
  pruneRuntime(profileDir)
  validateDefaultProfile(profileDir)
  return { seedHome, profileDir }
}
```

`createIsolatedNpmEnvironment` must strip inherited `npm_config_*`, prepend the staged Node directory to `Path`, point user/global config to empty staging files, and set the official registries plus `NPM_CONFIG_AUTO_INSTALL_PEERS=false`. The immutable GitHub tarball avoids npm/pnpm Git preparation of the plugin's local `link:` development dependencies; the normalized manifest retains the approved `github:` target. Hoisted pnpm output is audited to contain no reparse points.

- [ ] **Step 4: Integrate the builder before runtime audit/publish**

Keep the extracted npm CLI until both host and seed are complete:

```js
const { node, npmCli, extracted } = installNode(nodeDir, staging)
const dshBin = installDsh(node, npmCli, host, staging)
buildDefaultProfile({ node, npmCli, dshBin, staging })
rmSync(extracted, { recursive: true, force: true })
pruneRuntime(host)
auditRuntime(staging)
replaceRuntime(staging, runtime)
```

- [ ] **Step 5: Run Node tests and the real runtime build**

Run:

```powershell
node --test scripts/default-profile.test.mjs scripts/runtime-policy.test.mjs
node scripts/bundle-host.mjs
```

Expected: tests PASS; build reports five installed versions, seed at most 35 MiB, and complete runtime at most 260 MiB.

- [ ] **Step 6: Commit the builder**

```powershell
git add scripts/default-profile.mjs scripts/default-profile.test.mjs scripts/bundle-host.mjs
git commit -m "build: bundle five default plugins"
```

### Task 3: Seed only a missing profile at runtime

**Files:**
- Create: `src-tauri/src/profile_seed.rs`
- Create: `src-tauri/src/profile_seed/tests.rs`
- Modify: `src-tauri/src/lib.rs` (declare the module so focused tests compile; startup integration remains Task 4)

- [ ] **Step 1: Write failing Rust tests for the ownership boundary**

Use isolated temp roots and a valid seven-bundle fixture:

```rust
#[test]
fn seeds_a_missing_web_profile() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();

    let result = seed_new_web_profile(fixture.seed(), fixture.data()).unwrap();

    assert_eq!(result, SeedResult::Seeded);
    assert!(fixture.data().join("profiles/web/node_modules/dshmarket/package.json").exists());
}

#[test]
fn preserves_any_existing_web_path_byte_for_byte() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    let existing = fixture.data().join("profiles/web");
    fs::create_dir_all(&existing).unwrap();
    fs::write(existing.join("user.txt"), b"owned by user").unwrap();

    let result = seed_new_web_profile(fixture.seed(), fixture.data()).unwrap();

    assert_eq!(result, SeedResult::SkippedExisting);
    assert_eq!(fs::read(existing.join("user.txt")).unwrap(), b"owned by user");
}

#[test]
fn a_failed_copy_removes_only_owned_staging() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    let error = seed_with_copy(fixture.seed(), fixture.data(), |_, staging| {
        fs::create_dir_all(staging).unwrap();
        fs::write(staging.join("partial"), b"partial").unwrap();
        Err("injected copy failure".to_string())
    }).unwrap_err();
    assert!(error.contains("injected copy failure"));
    assert!(!fixture.data().join("profiles/web").exists());
    assert!(!fixture.data().join("profiles/.web.dsh-studio-seed").exists());
}
```

- [ ] **Step 2: Run the focused Rust test and verify RED**

Run: `cargo test profile_seed::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because the module and API do not exist.

- [ ] **Step 3: Implement validation, physical copying, cleanup, and atomic rename**

Expose only this narrow API:

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum SeedResult {
    Seeded,
    SkippedExisting,
}

pub fn seed_new_web_profile(seed: &Path, dsh_home: &Path) -> Result<SeedResult, String> {
    seed_with_copy(seed, dsh_home, copy_physical_tree)
}
```

The internal flow must early-return for any existing destination, validate exact dependencies/bundles/installed versions, reject `FILE_ATTRIBUTE_REPARSE_POINT`, remove only `profiles/.web.dsh-studio-seed`, copy files recursively with `fs::copy`, validate the copy, recheck destination absence, and publish with `fs::rename` on the same volume.

Add `mod profile_seed;` to `src-tauri/src/lib.rs` at this point so `cargo test profile_seed::tests` compiles the new module. Do not call the seeding API from startup until Task 4.

- [ ] **Step 4: Run profile seed and legacy migration tests**

Run:

```powershell
cargo test profile_seed::tests --manifest-path src-tauri/Cargo.toml
cargo test profile::tests --manifest-path src-tauri/Cargo.toml
```

Expected: both test groups PASS.

- [ ] **Step 5: Commit runtime seeding**

```powershell
git add src-tauri/src/profile_seed.rs src-tauri/src/profile_seed/tests.rs src-tauri/src/lib.rs
git commit -m "feat: seed default plugins for new profiles"
```

### Task 4: Connect seeding, resources, smoke tests, and documentation

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `scripts/runtime-policy.test.mjs`
- Modify: `scripts/smoke-test.mjs`
- Modify: `README.md`

- [ ] **Step 1: Add failing source/config tests**

Require the new resource and startup call:

```js
test('Tauri bundles the isolated default profile seed', () => {
  const config = JSON.parse(readFileSync(join(projectRoot, 'src-tauri', 'tauri.conf.json'), 'utf8'))
  assert.equal(config.version, '0.1.1')
  assert.equal(config.bundle.resources['../runtime/profile-seed'], 'runtime/profile-seed')
})

test('startup seeds after migration and before sidecar spawn', () => {
  const source = readFileSync(join(projectRoot, 'src-tauri', 'src', 'lib.rs'), 'utf8')
  const migrate = source.indexOf('migrate_legacy_web_profile')
  const seed = source.indexOf('seed_new_web_profile')
  const spawn = source.indexOf('sidecar::spawn')
  assert.ok(migrate < seed && seed < spawn)
})
```

- [ ] **Step 2: Run policy tests and verify RED**

Run: `node --test scripts/runtime-policy.test.mjs`

Expected: FAIL because the resource, version, and call are missing.

- [ ] **Step 3: Wire runtime seeding and bundled resources**

Add `mod profile_seed;` and invoke:

```rust
profile::migrate_legacy_web_profile(&data_dir)?;
profile_seed::seed_new_web_profile(
    &runtime_dir.join("profile-seed/profiles/web"),
    &data_dir,
)?;
```

Add the explicit Tauri resource mapping, update both Rust/Tauri versions to `0.1.1`, and regenerate the lockfile through Cargo rather than editing dependency locks manually.

- [ ] **Step 4: Make smoke testing seed-aware**

Use `DSH_SMOKE_USE_SEED=1` to copy `runtime/profile-seed` to a temporary home, require the exact seven bundles in `--dump-config`, reject `@linxin666`, then start the real HTTP server. Keep the default mode as a clean official-profile smoke so both paths remain testable.

- [ ] **Step 5: Update README with precise behavior and gates**

Document the five names/versions, offline first-launch behavior, the existing-profile skip rule, `node --test scripts/default-profile.test.mjs`, the 260/35/70 MiB budgets, and the fact that no removed enhanced UI is restored.

- [ ] **Step 6: Run focused and full tests**

Run:

```powershell
node --test scripts/default-profile.test.mjs scripts/runtime-policy.test.mjs
node scripts/smoke-test.mjs
$env:DSH_SMOKE_USE_SEED='1'; node scripts/smoke-test.mjs; Remove-Item Env:DSH_SMOKE_USE_SEED
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all commands PASS; seeded smoke reports seven bundles and HTTP 200-399.

- [ ] **Step 7: Commit integration and docs**

```powershell
git add src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock scripts/runtime-policy.test.mjs scripts/smoke-test.mjs README.md
git commit -m "feat: activate bundled plugins on fresh installs"
```

### Task 5: Release build, desktop verification, and review

**Files:**
- Verify: all modified files
- Produce: `src-tauri/target/release/bundle/nsis/dsh-studio_0.1.1_x64-setup.exe`

- [ ] **Step 1: Stop the old development instance cleanly**

Terminate the existing Tauri dev process through its owning terminal/session so Windows releases `runtime/node/node.exe`. Verify the desktop sidecar and port 7256 are gone before replacing `runtime`.

- [ ] **Step 2: Rebuild the clean runtime and verify budgets**

Run:

```powershell
node scripts/bundle-host.mjs
node --test scripts/default-profile.test.mjs scripts/runtime-policy.test.mjs
node scripts/smoke-test.mjs
$env:DSH_SMOKE_USE_SEED='1'; node scripts/smoke-test.mjs; Remove-Item Env:DSH_SMOKE_USE_SEED
```

Expected: runtime at most 260 MiB, seed at most 35 MiB, both smoke modes PASS.

- [ ] **Step 3: Run Rust quality gates**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: every command exits 0.

- [ ] **Step 4: Build and audit the NSIS installer**

Run:

```powershell
Push-Location src-tauri
cargo tauri build
Pop-Location
Get-Item src-tauri/target/release/bundle/nsis/dsh-studio_0.1.1_x64-setup.exe
Get-FileHash src-tauri/target/release/bundle/nsis/dsh-studio_0.1.1_x64-setup.exe -Algorithm SHA256
node --input-type=module -e "import('./scripts/runtime-policy.mjs').then(({auditInstaller}) => console.log(auditInstaller('./src-tauri/target/release/bundle/nsis/dsh-studio_0.1.1_x64-setup.exe')))"
```

Expected: installer exists and is no larger than 70 MiB; record its exact size and SHA256.

- [ ] **Step 5: Launch the new development desktop and inspect the seeded UI**

Run `cargo tauri dev` from `src-tauri`, keep it resident, verify the sidecar responds with HTTP 200, and confirm the profile manifest contains the five plugins. Use a temporary fresh app-data path or a controlled fresh profile so the existing user profile is not altered during this check.

- [ ] **Step 6: Perform code review and fix all critical/important findings**

Review the complete diff against `54dde52`, focusing on user-profile preservation, Windows atomicity/reparse handling, reproducible registries, package scripts, sidecar startup order, and size policy. Re-run affected tests after every fix.

- [ ] **Step 7: Commit the verified release state**

```powershell
git add scripts src-tauri README.md docs/superpowers/plans/2026-08-15-bundled-default-plugins.md
git commit -m "release: bundle default dsh plugins"
```

- [ ] **Step 8: Final evidence check**

Run:

```powershell
git status --short --branch
git log -5 --oneline
```

Expected: clean worktree on `codex/pure-dsh-web-desktop`, development desktop still running, and all verification evidence ready for handoff.
