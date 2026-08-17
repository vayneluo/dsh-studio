import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import { DEFAULT_BUNDLES, DEFAULT_PLUGINS } from './default-profile.mjs'

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(scriptsDir, '..')
const rustSource = readFileSync(join(projectRoot, 'src-tauri', 'src', 'profile_seed.rs'), 'utf8')

const pluginsBlock = rustSource.slice(
  rustSource.indexOf('const PLUGINS'),
  rustSource.indexOf('];', rustSource.indexOf('const PLUGINS')),
)
const bundlesBlock = rustSource.slice(
  rustSource.indexOf('const BUNDLES'),
  rustSource.indexOf('];', rustSource.indexOf('const BUNDLES')),
)

const rustPlugins = new Map()
for (const match of pluginsBlock.matchAll(/\(\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)"\s*,?\s*\)/gs)) {
  rustPlugins.set(match[1], { version: match[2], spec: match[3] })
}

const rustBundles = [...bundlesBlock.matchAll(/"([^"]+)"/g)].map((match) => match[1])

test('Rust profile_seed plugin catalog matches the JS DEFAULT_PLUGINS catalog', () => {
  const jsCatalog = new Map(
    DEFAULT_PLUGINS.map((plugin) => [
      plugin.packageName,
      {
        version: plugin.version,
        // Rust stores the package.json dependency spec; for the GitHub plugin
        // that is the pinned tarball URL, for registry plugins it is the version.
        spec: plugin.packageName === 'dsh-at-file' ? plugin.target : plugin.version,
      },
    ]),
  )

  const rustNames = [...rustPlugins.keys()].sort()
  const jsNames = [...jsCatalog.keys()].sort()
  assert.deepEqual(
    rustNames,
    jsNames,
    'plugin names differ between src-tauri/src/profile_seed.rs and scripts/default-profile.mjs',
  )

  for (const name of rustNames) {
    assert.deepEqual(
      rustPlugins.get(name),
      jsCatalog.get(name),
      `plugin catalog mismatch for ${name}; update BOTH profile_seed.rs and default-profile.mjs`,
    )
  }
})

test('Rust profile_seed bundle order matches the JS DEFAULT_BUNDLES order', () => {
  assert.deepEqual(
    rustBundles,
    DEFAULT_BUNDLES,
    'bundle order differs between src-tauri/src/profile_seed.rs and scripts/default-profile.mjs',
  )
})
