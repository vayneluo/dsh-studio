import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { delimiter, dirname, join, relative } from 'node:path'

import { auditPortableTree, pruneRuntime } from './runtime-policy.mjs'

const NPM_REGISTRY = 'https://registry.npmjs.org/'
const AT_FILE_TARBALL = 'https://github.com/omdsh-dev/dsh-at-file/archive/e579d0deb2295d5fea37a89244f8d584999be850.tar.gz'

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

const packageSpecs = (plugin) => plugin.packageName === 'dsh-at-file'
  ? { dependency: plugin.target, install: AT_FILE_TARBALL }
  : { dependency: plugin.version, install: plugin.target }

const expectedDependencySpec = (plugin) => packageSpecs(plugin).dependency
const installTarget = (plugin) => packageSpecs(plugin).install

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'))

const sameArray = (actual, expected) => (
  Array.isArray(actual)
  && actual.length === expected.length
  && actual.every((value, index) => value === expected[index])
)

export const validateDefaultProfileConfig = (config) => {
  if (config.includes('@linxin666')) {
    throw new Error('enhanced Web UI leaked into the seeded runtime config')
  }
  const headers = [...config.matchAll(/^# == ([^,\r\n]+)\r?$/gm)]
    .map((match) => match[1])
  const uniqueHeaders = [...new Set(headers)]
  if (!sameArray(uniqueHeaders, DEFAULT_BUNDLES)) {
    throw new Error(`seeded runtime bundle headers do not match the expected order: ${uniqueHeaders.join(', ')}`)
  }
  return uniqueHeaders
}

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

const runCommand = (command, args, options = {}) => {
  console.log('>', command, args.join(' '))
  execFileSync(command, args, { stdio: 'inherit', ...options })
}

const normalizeProfileManifest = (profileDir) => {
  const manifestPath = join(profileDir, 'package.json')
  const manifest = readJson(manifestPath)
  const gitPlugin = DEFAULT_PLUGINS.find(({ packageName }) => packageName === 'dsh-at-file')
  manifest.dependencies[gitPlugin.packageName] = expectedDependencySpec(gitPlugin)
  manifest.dsh.profile.bundles = [...DEFAULT_BUNDLES]
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
}

const catalogFingerprint = (profileDir) => {
  const hash = createHash('sha256')
  const pending = [profileDir]
  while (pending.length > 0) {
    const directory = pending.pop()
    const entries = readdirSync(directory, { withFileTypes: true })
      .sort((left, right) => left.name.localeCompare(right.name))
    for (const entry of entries) {
      const path = join(directory, entry.name)
      if (entry.isDirectory()) {
        pending.push(path)
        continue
      }
      if (!entry.isFile()) {
        throw new Error(`managed profile contains an unsupported path: ${path}`)
      }
      hash.update(relative(profileDir, path).replaceAll('\\', '/'))
      hash.update('\0')
      hash.update(readFileSync(path))
      hash.update('\0')
    }
  }
  return hash.digest('hex')
}

const writeManagedMarker = (profileDir) => {
  writeFileSync(
    join(profileDir, '.dsh-studio-managed.json'),
    `${JSON.stringify({
      catalogVersion: 1,
      catalogFingerprint: catalogFingerprint(profileDir),
    }, null, 2)}\n`,
  )
}

const createIsolatedNpmEnvironment = (staging, node, baseEnv) => {
  const env = Object.fromEntries(
    Object.entries(baseEnv).filter(([name]) => !name.toLowerCase().startsWith('npm_config_')),
  )
  const pathKey = Object.keys(env).find((name) => name.toLowerCase() === 'path') ?? 'Path'
  const userConfig = join(staging, 'default-profile-npm-user.ini')
  const globalConfig = join(staging, 'default-profile-npm-global.ini')
  mkdirSync(staging, { recursive: true })
  writeFileSync(userConfig, '')
  writeFileSync(globalConfig, '')
  env[pathKey] = `${dirname(node)}${delimiter}${env[pathKey] ?? ''}`
  env.NPM_CONFIG_USERCONFIG = userConfig
  env.NPM_CONFIG_GLOBALCONFIG = globalConfig
  env.NPM_CONFIG_REGISTRY = NPM_REGISTRY
  env.NPM_CONFIG_AUTO_INSTALL_PEERS = 'false'
  return { env, globalConfig, userConfig }
}

export const buildDefaultProfile = ({
  node,
  dshBin,
  staging,
  baseEnv = process.env,
  run = runCommand,
}) => {
  const seedHome = join(staging, 'profile-seed')
  const profileDir = join(seedHome, 'profiles', 'web')
  const isolated = createIsolatedNpmEnvironment(staging, node, baseEnv)

  try {
    run(node, [
      dshBin,
      'plugin',
      '--profile',
      'web',
      'add',
      '-w',
      '--config.node-linker=hoisted',
      '--config.auto-install-peers=false',
      ...DEFAULT_PLUGINS.map(installTarget),
    ], { env: { ...isolated.env, DSH_HOME: seedHome } })

    normalizeProfileManifest(profileDir)
    rmSync(join(profileDir, 'node_modules', '.pnpm'), { recursive: true, force: true })
    rmSync(join(profileDir, 'node_modules', '.bin'), { recursive: true, force: true })
    rmSync(join(profileDir, 'node_modules', '.modules.yaml'), { force: true })
    rmSync(join(profileDir, 'pnpm-lock.yaml'), { force: true })
    const pruned = pruneRuntime(profileDir)
    writeManagedMarker(profileDir)
    const audit = validateDefaultProfile(profileDir)
    return { audit, profileDir, pruned, seedHome }
  } finally {
    rmSync(isolated.userConfig, { force: true })
    rmSync(isolated.globalConfig, { force: true })
  }
}
