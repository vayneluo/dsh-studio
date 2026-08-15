#!/usr/bin/env node
// Build the distributable Windows x64 runtime: Node + official DSH only.
// Build-machine requirements: curl, unzip, and npm. End users need none of them.
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, renameSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { auditRuntime, pruneRuntime } from './runtime-policy.mjs'

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const root = join(scriptsDir, '..')
const runtime = join(root, 'runtime')
const host = join(runtime, 'host')
const nodeDir = join(runtime, 'node')
const NODE_VERSION = '24.13.0'
const DSH_VERSION = '0.1.0-rc.6'

const run = (command, args, options = {}) => {
  console.log('>', command, args.join(' '))
  execFileSync(command, args, { stdio: 'inherit', ...options })
}

const ensureNode = () => {
  const node = join(nodeDir, 'node.exe')
  if (existsSync(node)) return node

  mkdirSync(nodeDir, { recursive: true })
  const zip = join(runtime, 'node.zip')
  run('curl', ['-sSL', '-o', zip, `https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip`])
  run('unzip', ['-o', '-q', zip, '-d', nodeDir])
  const extracted = join(nodeDir, `node-v${NODE_VERSION}-win-x64`)
  renameSync(join(extracted, 'node.exe'), node)
  rmSync(extracted, { recursive: true, force: true })
  rmSync(zip, { force: true })
  return node
}

const ensureDsh = () => {
  const dshBin = join(host, 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
  if (existsSync(dshBin)) return dshBin

  mkdirSync(host, { recursive: true })
  run('npm', ['install', '--prefix', host, `@deepseek-ai/dsh@${DSH_VERSION}`])
  return dshBin
}

const verifyOfficialProfile = (node, dshBin) => {
  const home = mkdtempSync(join(tmpdir(), 'dsh-studio-bundle-check-'))
  try {
    const config = execFileSync(node, [dshBin, 'web', '--dump-default-config'], {
      encoding: 'utf8',
      env: { ...process.env, DSH_HOME: home },
    })
    if (!config.includes('@deepseek-ai/dsh-web-app')) {
      throw new Error('Official DSH Web profile is missing its application bundle')
    }
  } finally {
    rmSync(home, { recursive: true, force: true })
  }
}

const node = ensureNode()
const dshBin = ensureDsh()

// Remove generated state left by older builds. DSH creates its official Web profile
// in the per-user app data directory on first launch.
rmSync(join(runtime, 'home'), { recursive: true, force: true })

const pruned = pruneRuntime(host)
verifyOfficialProfile(node, dshBin)
const audit = auditRuntime(runtime)

console.log(`Pruned ${pruned.removedFiles} files (${(pruned.removedBytes / 1024 / 1024).toFixed(2)} MiB)`)
console.log(`Runtime ${audit.files} files, ${(audit.bytes / 1024 / 1024).toFixed(2)} MiB`)
console.log('bundle-host done')
