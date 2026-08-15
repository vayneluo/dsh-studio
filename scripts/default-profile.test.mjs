import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
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

const makeValidProfile = (t) => {
  const profile = mkdtempSync(join(tmpdir(), 'dsh-default-profile-test-'))
  t.after(() => rmSync(profile, { recursive: true, force: true }))
  writeFileSync(join(profile, 'package.json'), JSON.stringify({
    name: 'dsh-profile-web',
    private: true,
    dependencies: EXPECTED_DEPENDENCIES,
    dsh: {
      profile: {
        bundles: [
          '@deepseek-ai/dsh-base',
          '@deepseek-ai/dsh-web-app',
          'dsh-at-file',
          '@liustack/modlens',
          'dsh-better-sidebar',
          'dshmarket',
          'dsh-message-edit',
        ],
      },
    },
  }))
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
