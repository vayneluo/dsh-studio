import assert from 'node:assert/strict'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

const EXPECTED_DEPENDENCIES = {
  '@liustack/modlens': '3.16.6',
  'dsh-at-file': 'github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850',
  'dsh-better-sidebar': '0.12.1',
  'dsh-message-edit': '0.2.1',
  dshmarket: '1.2.2',
}

const AT_FILE_TARBALL = 'https://github.com/omdsh-dev/dsh-at-file/archive/e579d0deb2295d5fea37a89244f8d584999be850.tar.gz'

const EXPECTED_BUNDLES = [
  '@deepseek-ai/dsh-base',
  '@deepseek-ai/dsh-web-app',
  'dsh-at-file',
  '@liustack/modlens',
  'dsh-better-sidebar',
  'dshmarket',
  'dsh-message-edit',
]

const writeInstalledPackages = (profile) => {
  for (const [packageName, version] of [
    ['dsh-at-file', '0.6.0'],
    ['@liustack/modlens', '3.16.6'],
    ['dsh-better-sidebar', '0.12.1'],
    ['dshmarket', '1.2.2'],
    ['dsh-message-edit', '0.2.1'],
  ]) {
    const packageDir = join(profile, 'node_modules', ...packageName.split('/'))
    mkdirSync(packageDir, { recursive: true })
    writeFileSync(join(packageDir, 'package.json'), JSON.stringify({ name: packageName, version }))
  }
}

const writeValidProfile = (profile, dependencies = EXPECTED_DEPENDENCIES, bundles = EXPECTED_BUNDLES) => {
  mkdirSync(profile, { recursive: true })
  writeFileSync(join(profile, 'package.json'), JSON.stringify({
    name: 'dsh-profile-web',
    private: true,
    dependencies,
    dsh: {
      profile: {
        bundles,
      },
    },
  }))
  writeInstalledPackages(profile)
}

const makeValidProfile = (t) => {
  const profile = mkdtempSync(join(tmpdir(), 'dsh-default-profile-test-'))
  t.after(() => rmSync(profile, { recursive: true, force: true }))
  writeValidProfile(profile)
  return profile
}

test('default plugin catalog is fixed and reproducible', async () => {
  await assert.doesNotReject(async () => {
    const { DEFAULT_BUNDLES, DEFAULT_PLUGINS } = await import('./default-profile.mjs')

    assert.deepEqual(DEFAULT_PLUGINS, [
      {
        bundle: 'dsh-at-file',
        packageName: 'dsh-at-file',
        version: '0.6.0',
        target: 'github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850',
      },
      {
        bundle: '@liustack/modlens',
        packageName: '@liustack/modlens',
        version: '3.16.6',
        target: '@liustack/modlens@3.16.6',
      },
      {
        bundle: 'dsh-better-sidebar',
        packageName: 'dsh-better-sidebar',
        version: '0.12.1',
        target: 'dsh-better-sidebar@0.12.1',
      },
      {
        bundle: 'dshmarket',
        packageName: 'dshmarket',
        version: '1.2.2',
        target: 'dshmarket@1.2.2',
      },
      {
        bundle: 'dsh-message-edit',
        packageName: 'dsh-message-edit',
        version: '0.2.1',
        target: 'dsh-message-edit@0.2.1',
      },
    ])
    assert.deepEqual(DEFAULT_BUNDLES, [
      '@deepseek-ai/dsh-base',
      '@deepseek-ai/dsh-web-app',
      'dsh-at-file',
      '@liustack/modlens',
      'dsh-better-sidebar',
      'dshmarket',
      'dsh-message-edit',
    ])
  })
})

test('validates the exact seven-bundle profile and installed versions', async (t) => {
  const profile = makeValidProfile(t)
  const { validateDefaultProfile } = await import('./default-profile.mjs')

  assert.doesNotThrow(() => validateDefaultProfile(profile))
})

test('rejects an extra direct dependency', async (t) => {
  const profile = makeValidProfile(t)
  const manifestPath = join(profile, 'package.json')
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
  manifest.dependencies['unexpected-plugin'] = '1.0.0'
  writeFileSync(manifestPath, JSON.stringify(manifest))
  const { validateDefaultProfile } = await import('./default-profile.mjs')

  assert.throws(() => validateDefaultProfile(profile), /dependencies/i)
})

test('rejects a non-portable junction in the seed', async (t) => {
  const profile = makeValidProfile(t)
  const target = join(profile, 'physical-target')
  mkdirSync(target)
  symlinkSync(target, join(profile, 'linked-target'), 'junction')
  const { validateDefaultProfile } = await import('./default-profile.mjs')

  assert.throws(() => validateDefaultProfile(profile), /reparse|symbolic|junction/i)
})

test('rejects an installed package version mismatch', async (t) => {
  const profile = makeValidProfile(t)
  writeFileSync(
    join(profile, 'node_modules', 'dshmarket', 'package.json'),
    JSON.stringify({ name: 'dshmarket', version: '9.9.9' }),
  )
  const { validateDefaultProfile } = await import('./default-profile.mjs')

  assert.throws(() => validateDefaultProfile(profile), /dshmarket.*version/i)
})

test('requires the exact runtime bundle headers in order', async () => {
  const { validateDefaultProfileConfig } = await import('./default-profile.mjs')
  const valid = [
    '# == @deepseek-ai/dsh-base',
    '# == @deepseek-ai/dsh-base',
    '# == @deepseek-ai/dsh-base, patched by @deepseek-ai/dsh-web-app',
    '# == @deepseek-ai/dsh-web-app',
    '# == dsh-at-file',
    '# == @liustack/modlens',
    '# == dsh-better-sidebar',
    '# == dshmarket',
    '# == dsh-message-edit',
  ].join('\n')

  assert.equal(typeof validateDefaultProfileConfig, 'function')
  assert.doesNotThrow(() => validateDefaultProfileConfig(valid))
  assert.throws(
    () => validateDefaultProfileConfig(`${valid}\n# == unexpected-plugin`),
    /runtime bundle headers/i,
  )
  assert.throws(
    () => validateDefaultProfileConfig(valid.replace(
      '# == dsh-at-file\n# == @liustack/modlens',
      '# == @liustack/modlens\n# == dsh-at-file',
    )),
    /runtime bundle headers/i,
  )
})

test('builds a portable hoisted seed from the immutable GitHub tarball', async (t) => {
  const staging = mkdtempSync(join(tmpdir(), 'dsh-default-profile-build-test-'))
  t.after(() => rmSync(staging, { recursive: true, force: true }))
  const calls = []
  const profile = join(staging, 'profile-seed', 'profiles', 'web')
  const { buildDefaultProfile } = await import('./default-profile.mjs')

  assert.equal(typeof buildDefaultProfile, 'function')
  const result = buildDefaultProfile({
    node: 'C:\\staged-node\\node.exe',
    dshBin: 'C:\\staged-host\\dsh.js',
    staging,
    baseEnv: {
      Path: 'C:\\Windows',
      NPM_CONFIG_REGISTRY: 'http://npm.invalid.local/',
      npm_config_proxy: 'http://proxy.invalid.local/',
      SAFE_VALUE: 'kept',
    },
    run: (command, args, options) => {
      calls.push({ command, args, env: options.env })
      writeValidProfile(
        profile,
        { ...EXPECTED_DEPENDENCIES, 'dsh-at-file': AT_FILE_TARBALL },
        [
          '@deepseek-ai/dsh-base',
          '@deepseek-ai/dsh-web-app',
          '@liustack/modlens',
          'dsh-at-file',
          'dsh-better-sidebar',
          'dsh-message-edit',
          'dshmarket',
        ],
      )
      writeInstalledPackages(profile)
      mkdirSync(join(profile, 'node_modules', '.pnpm'), { recursive: true })
      writeFileSync(join(profile, 'node_modules', '.pnpm', 'lock.yaml'), 'lockfileVersion: 9')
      mkdirSync(join(profile, 'node_modules', '.bin'), { recursive: true })
      writeFileSync(join(profile, 'node_modules', '.bin', 'unused.cmd'), 'unused')
      writeFileSync(join(profile, 'node_modules', 'dshmarket', 'debug.pdb'), 'debug')
    },
  })

  assert.equal(result.profileDir, profile)
  assert.equal(calls.length, 1)
  assert.equal(calls[0].command, 'C:\\staged-node\\node.exe')
  assert.deepEqual(calls[0].args.slice(0, 6), [
    'C:\\staged-host\\dsh.js', 'plugin', '--profile', 'web', 'add', '-w',
  ])
  assert.ok(calls[0].args.includes('--config.node-linker=hoisted'))
  assert.ok(calls[0].args.includes('--config.auto-install-peers=false'))
  assert.ok(calls[0].args.includes(AT_FILE_TARBALL))
  for (const call of calls) {
    assert.equal(call.env.NPM_CONFIG_REGISTRY, 'https://registry.npmjs.org/')
    assert.equal(call.env.NPM_CONFIG_AUTO_INSTALL_PEERS, 'false')
    assert.equal(call.env.SAFE_VALUE, 'kept')
    assert.equal(Object.values(call.env).includes('http://npm.invalid.local/'), false)
    assert.equal(Object.values(call.env).includes('http://proxy.invalid.local/'), false)
  }
  assert.equal(existsSync(join(profile, 'node_modules', '.pnpm')), false)
  assert.equal(existsSync(join(profile, 'node_modules', '.bin')), false)
  assert.equal(existsSync(join(profile, 'node_modules', 'dshmarket', 'debug.pdb')), false)
  assert.deepEqual(JSON.parse(readFileSync(join(profile, 'package.json'), 'utf8')).dependencies, EXPECTED_DEPENDENCIES)
  const marker = JSON.parse(readFileSync(join(profile, '.dsh-studio-managed.json'), 'utf8'))
  assert.equal(marker.catalogVersion, 1)
  assert.match(marker.catalogFingerprint, /^[a-f0-9]{64}$/)
  assert.equal(existsSync(join(staging, 'default-profile-npm-user.ini')), false)
  assert.equal(existsSync(join(staging, 'default-profile-npm-global.ini')), false)
})
