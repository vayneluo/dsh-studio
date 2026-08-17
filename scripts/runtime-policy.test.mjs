import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import * as runtimePolicy from './runtime-policy.mjs'

const {
  auditInstaller,
  auditPortableTree,
  auditRuntime,
  DEFAULT_INSTALLER_BUDGET_BYTES,
  DEFAULT_PROFILE_BUDGET_BYTES,
  DEFAULT_RUNTIME_BUDGET_BYTES,
  pruneRuntime,
  replaceRuntime,
} = runtimePolicy

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(scriptsDir, '..')

const makeFixture = () => mkdtempSync(join(tmpdir(), 'dsh-runtime-policy-'))

const writeFixture = (root, relativePath, contents = relativePath) => {
  const path = join(root, relativePath)
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, contents)
  return path
}

test('pruneRuntime keeps Windows x64 runtime files and removes non-runtime assets', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))

  const host = join(root, 'host')
  const files = {
    keep: writeFixture(host, 'node_modules/example/keep.js'),
    license: writeFixture(host, 'node_modules/example/LICENSE'),
    pdb: writeFixture(host, 'node_modules/example/debug.pdb'),
    map: writeFixture(host, 'node_modules/example/code.js.map'),
    dts: writeFixture(host, 'node_modules/example/types.d.ts'),
    dmts: writeFixture(host, 'node_modules/example/types.d.mts'),
    dcts: writeFixture(host, 'node_modules/example/types.d.cts'),
    winPty: writeFixture(host, 'node_modules/node-pty/prebuilds/win32-x64/pty.node'),
    armPty: writeFixture(host, 'node_modules/node-pty/prebuilds/win32-arm64/pty.node'),
    darwinPty: writeFixture(host, 'node_modules/node-pty/prebuilds/darwin-x64/pty.node'),
    source: writeFixture(host, 'node_modules/node-pty/src/windowsPtyAgent.ts'),
    genericSource: writeFixture(host, 'node_modules/example/src/index.ts'),
    genericWinNative: writeFixture(host, 'node_modules/example/prebuilds/win32-x64/native.node'),
    genericLinuxNative: writeFixture(host, 'node_modules/example/prebuilds/linux-x64/native.node'),
    genericDarwinNative: writeFixture(host, 'node_modules/example/prebuilds/darwin-x64/native.node'),
    genericSharedObject: writeFixture(host, 'node_modules/example/native/addon.so'),
  }

  const result = pruneRuntime(host)

  assert.equal(result.removedFiles > 0, true)
  assert.equal(result.removedBytes > 0, true)
  assert.equal(auditRuntime(root, { maxBytes: 1024 }).files > 0, true)
  assert.equal(auditRuntime(root, { maxBytes: 1024 }).forbiddenPaths.length, 0)

  const kept = new Set(auditRuntime(root, { maxBytes: 1024 }).paths)
  assert.equal(kept.has(files.keep), true)
  assert.equal(kept.has(files.license), true)
  assert.equal(kept.has(files.winPty), true)
  assert.equal(kept.has(files.genericWinNative), true)
  for (const path of [files.pdb, files.map, files.dts, files.dmts, files.dcts, files.armPty, files.darwinPty, files.source, files.genericSource, files.genericLinuxNative, files.genericDarwinNative, files.genericSharedObject]) {
    assert.equal(kept.has(path), false, `${path} should be removed`)
  }
})

test('auditRuntime rejects third-party enhanced Web UI packages', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  writeFixture(root, 'host/node_modules/@linxin666/dsh-web-ui-all/package.json', '{}')

  assert.throws(
    () => auditRuntime(root),
    /forbidden runtime path.*@linxin666/i,
  )
})

test('auditRuntime enforces the expanded runtime byte budget', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  writeFixture(root, 'host/large.bin', Buffer.alloc(64))

  assert.throws(
    () => auditRuntime(root, { maxBytes: 32 }),
    /runtime budget exceeded/i,
  )
})

test('release budgets allow a 35 MiB seed, 260 MiB runtime, and 70 MiB installer', () => {
  assert.equal(DEFAULT_PROFILE_BUDGET_BYTES, 35 * 1024 * 1024)
  assert.equal(DEFAULT_RUNTIME_BUDGET_BYTES, 260 * 1024 * 1024)
  assert.equal(DEFAULT_INSTALLER_BUDGET_BYTES, 70 * 1024 * 1024)
})

test('auditPortableTree rejects junctions and symbolic links', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const target = join(root, 'store', 'plugin')
  writeFixture(target, 'package.json', '{}')
  const seed = join(root, 'seed')
  mkdirSync(join(seed, 'node_modules'), { recursive: true })
  symlinkSync(target, join(seed, 'node_modules', 'plugin'), 'junction')

  assert.throws(() => auditPortableTree(seed), /reparse|symbolic|junction/i)
})

test('auditPortableTree enforces the profile byte budget', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  writeFixture(root, 'large.bin', Buffer.alloc(64))

  assert.throws(() => auditPortableTree(root, { maxBytes: 32 }), /profile budget exceeded/i)
})

test('auditPortableTree rejects the removed enhanced Web UI', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  writeFixture(root, 'node_modules/@linxin666/dsh-web-ui-all/package.json', '{}')

  assert.throws(() => auditPortableTree(root), /forbidden.*@linxin666/i)
})

test('auditInstaller enforces the compressed installer byte budget', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const installer = writeFixture(root, 'setup.exe', Buffer.alloc(64))

  assert.throws(() => auditInstaller(installer, { maxBytes: 32 }), /installer budget exceeded/i)
  assert.equal(auditInstaller(installer, { maxBytes: 64 }).bytes, 64)
})

test('replaceRuntime swaps in a clean staging tree without stale files', (t) => {
  const root = makeFixture()
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const runtime = join(root, 'runtime')
  const staging = join(root, '.runtime-stage-test')
  writeFixture(runtime, 'host/stale-package/package.json', '{}')
  writeFixture(staging, 'host/node_modules/@deepseek-ai/dsh/package.json', '{}')
  writeFixture(staging, 'node/node.exe', 'node')

  replaceRuntime(staging, runtime)

  assert.equal(readFileSync(join(runtime, 'node/node.exe'), 'utf8'), 'node')
  assert.equal(readFileSync(join(runtime, 'host/node_modules/@deepseek-ai/dsh/package.json'), 'utf8'), '{}')
  assert.throws(() => readFileSync(join(runtime, 'host/stale-package/package.json')), /ENOENT/)
  assert.throws(() => readFileSync(join(staging, 'node/node.exe')), /ENOENT/)
})

test('bundle host builds the official DSH runtime and isolated default profile seed', () => {
  const source = readFileSync(join(scriptsDir, 'bundle-host.mjs'), 'utf8')

  assert.doesNotMatch(source, /@linxin666|dsh-web-ui|dsh-skins/)
  assert.doesNotMatch(source, /run\(['"]pnpm['"]/, 'runtime builds must not require a global pnpm executable')
  assert.match(source, /import \{ buildDefaultProfile \} from ['"]\.\/default-profile\.mjs['"]/)
  assert.match(source, /import \{ applyProviderOnboardingOverride \} from ['"]\.\/provider-onboarding-override\.mjs['"]/)
  assert.match(source, /buildDefaultProfile\(\{ node, dshBin, staging \}\)/)
  const install = source.indexOf('installDsh(')
  const override = source.indexOf('applyProviderOnboardingOverride(host)')
  const profile = source.indexOf('buildDefaultProfile(')
  const prune = source.indexOf('pruneRuntime(host)')
  assert.ok(install < override && override < profile && profile < prune)
  assert.ok(source.indexOf('buildDefaultProfile(') < source.indexOf('rmSync(extracted,'))
  assert.match(source, /pruneRuntime\(host\)/)
  assert.match(source, /auditRuntime\(staging\)/)
  assert.match(source, /replaceRuntime\(staging, runtime\)/)
  assert.doesNotMatch(source, /run\(['"]unzip['"]/, 'Windows builds must not require an external unzip executable')
  assert.match(source, /Expand-Archive/)
  assert.match(source, /DSH_NODE_ZIP/)
  assert.match(source, /DSH_NODE_DIR/)
  assert.doesNotMatch(source, /run\(['"]npm['"]/, 'runtime builds must not require a global npm executable')
  assert.match(source, /npm-cli\.js/)
  assert.match(source, /https:\/\/registry\.npmjs\.org\//)
  assert.match(source, /--@deepseek-ai:registry=https:\/\/registry\.npmjs\.org\//)
  assert.match(source, /NPM_CONFIG_USERCONFIG/)
  assert.match(source, /npmEnv\[pathKey\][^\n]*dirname\(node\)/)
  assert.match(source, /rmSync\(userConfig/)
  assert.match(source, /rmSync\(globalConfig/)
  assert.match(source, /smoke-test\.mjs/)
  assert.match(source, /DSH_SMOKE_RUNTIME_DIR:\s*staging/)
  assert.match(source, /DSH_SMOKE_USE_SEED:\s*['"]1['"]/)
  assert.ok(source.indexOf('DSH_SMOKE_USE_SEED') < source.indexOf('auditRuntime(staging)'))
})

test('startup page exposes a text-only error state', () => {
  const source = readFileSync(join(projectRoot, 'ui', 'index.html'), 'utf8')

  assert.match(source, /id=["']startup-error["']/)
  assert.match(source, /window\.showStartupError/)
  assert.match(source, /__dshStartupError/)
  assert.match(source, /startup_ui_ready/)
  assert.match(source, /textContent/)
  assert.doesNotMatch(source, /innerHTML/)
})

test('startup page uses DS Studio product copy', () => {
  const source = readFileSync(join(projectRoot, 'ui', 'index.html'), 'utf8')

  assert.match(source, /<title>DS Studio<\/title>/)
  assert.match(source, /<h1>DS<br>STUDIO<\/h1>/)
  assert.match(source, /正在启动 DS Studio…/)
  assert.match(source, /DS Studio 未能完成启动。/)
  assert.doesNotMatch(source, /DSH<br>STUDIO/)
  assert.doesNotMatch(source, /正在启动官方 DSH Web/)
  assert.doesNotMatch(source, /DSH host 未能完成启动。/)
})

test('Windows executable uses the GUI subsystem without a console window', () => {
  const source = readFileSync(join(projectRoot, 'src-tauri', 'src', 'main.rs'), 'utf8')
  assert.match(source, /windows_subsystem\s*=\s*"windows"/)
})

test('Tauri bundles the official runtime and isolated default profile seed at version 0.1.4', () => {
  const config = JSON.parse(
    readFileSync(join(projectRoot, 'src-tauri', 'tauri.conf.json'), 'utf8'),
  )

  assert.equal(config.version, '0.1.4')
  assert.deepEqual(config.bundle.resources, {
    '../runtime/node/node.exe': 'runtime/node/node.exe',
    '../runtime/host': 'runtime/host',
    '../runtime/profile-seed': 'runtime/profile-seed',
  })
  assert.equal(config.app.withGlobalTauri, true)
})

test('bundled default profile carries a content fingerprint for managed upgrades', () => {
  const marker = JSON.parse(readFileSync(
    join(projectRoot, 'runtime', 'profile-seed', 'profiles', 'web', '.dsh-studio-managed.json'),
    'utf8',
  ))

  assert.equal(marker.catalogVersion, 1)
  assert.match(marker.catalogFingerprint, /^[a-f0-9]{64}$/)
})

test('startup seeds after legacy migration and before sidecar spawn', () => {
  const source = readFileSync(join(projectRoot, 'src-tauri', 'src', 'lib.rs'), 'utf8')
  const migrate = source.indexOf('migrate_legacy_web_profile')
  const seed = source.indexOf('seed_new_web_profile')
  const spawn = source.indexOf('sidecar::spawn')

  assert.ok(migrate >= 0)
  assert.ok(migrate < seed && seed < spawn)
})

test('HTTP smoke can exercise and validate the bundled profile seed', () => {
  const source = readFileSync(join(scriptsDir, 'smoke-test.mjs'), 'utf8')

  assert.match(source, /DSH_SMOKE_USE_SEED/)
  assert.match(source, /runtimeDir, ['"]profile-seed/)
  assert.match(source, /validateDefaultProfile/)
  assert.match(source, /validateDefaultProfileConfig/)
})
