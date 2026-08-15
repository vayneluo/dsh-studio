import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { auditPortableTree } from './runtime-policy.mjs'

export const DEFAULT_PLUGINS = Object.freeze([
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

export const DEFAULT_BUNDLES = Object.freeze([
  '@deepseek-ai/dsh-base',
  '@deepseek-ai/dsh-web-app',
  ...DEFAULT_PLUGINS.map(({ bundle }) => bundle),
])

const expectedDependencySpec = (plugin) => (
  plugin.packageName === 'dsh-at-file' ? plugin.target : plugin.version
)

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'))

const sameArray = (actual, expected) => (
  Array.isArray(actual)
  && actual.length === expected.length
  && actual.every((value, index) => value === expected[index])
)

export const validateDefaultProfile = (profileDir) => {
  const manifest = readJson(join(profileDir, 'package.json'))
  const dependencies = manifest.dependencies ?? {}
  const dependencyNames = Object.keys(dependencies).sort()
  const expectedNames = DEFAULT_PLUGINS.map(({ packageName }) => packageName).sort()

  if (!sameArray(dependencyNames, expectedNames)) {
    throw new Error('default profile dependencies do not match the fixed plugin catalog')
  }
  for (const plugin of DEFAULT_PLUGINS) {
    if (dependencies[plugin.packageName] !== expectedDependencySpec(plugin)) {
      throw new Error(`default profile dependency spec mismatch for ${plugin.packageName}`)
    }
  }
  if (!sameArray(manifest.dsh?.profile?.bundles, DEFAULT_BUNDLES)) {
    throw new Error('default profile bundles do not match the expected seven-bundle order')
  }

  for (const plugin of DEFAULT_PLUGINS) {
    const packageManifest = readJson(join(
      profileDir,
      'node_modules',
      ...plugin.packageName.split('/'),
      'package.json',
    ))
    if (packageManifest.name !== plugin.packageName) {
      throw new Error(`installed package name mismatch for ${plugin.packageName}`)
    }
    if (packageManifest.version !== plugin.version) {
      throw new Error(`${plugin.packageName} version mismatch: expected ${plugin.version}, got ${packageManifest.version}`)
    }
  }

  const audit = auditPortableTree(profileDir)
  return {
    ...audit,
    dependencies: dependencyNames.length,
    bundles: DEFAULT_BUNDLES.length,
  }
}
