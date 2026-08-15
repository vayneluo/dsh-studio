#!/usr/bin/env node
// Build the distributable Windows x64 runtime: Node + official DSH only.
// Build-machine requirements: curl and Windows PowerShell. End users need neither.
import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, renameSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { delimiter, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { auditRuntime, pruneRuntime, replaceRuntime } from './runtime-policy.mjs'

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const root = join(scriptsDir, '..')
const runtime = join(root, 'runtime')
const NODE_VERSION = '24.13.0'
const DSH_VERSION = '0.1.0-rc.6'
const NPM_REGISTRY = 'https://registry.npmjs.org/'

const run = (command, args, options = {}) => {
  console.log('>', command, args.join(' '))
  execFileSync(command, args, { stdio: 'inherit', ...options })
}

const installNode = (nodeDir, staging) => {
  const node = join(nodeDir, 'node.exe')
  mkdirSync(nodeDir, { recursive: true })
  const zip = join(staging, 'node.zip')
  run('curl', ['-sSL', '-o', zip, `https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip`])
  run('powershell.exe', [
    '-NoProfile',
    '-NonInteractive',
    '-Command',
    'Expand-Archive -LiteralPath $env:DSH_NODE_ZIP -DestinationPath $env:DSH_NODE_DIR -Force',
  ], {
    env: { ...process.env, DSH_NODE_ZIP: zip, DSH_NODE_DIR: nodeDir },
  })
  const extracted = join(nodeDir, `node-v${NODE_VERSION}-win-x64`)
  const npmCli = join(extracted, 'node_modules', 'npm', 'bin', 'npm-cli.js')
  renameSync(join(extracted, 'node.exe'), node)
  rmSync(zip, { force: true })
  return { node, npmCli, extracted }
}

const installDsh = (node, npmCli, host, staging) => {
  const dshBin = join(host, 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
  const userConfig = join(staging, 'npm-user.ini')
  const globalConfig = join(staging, 'npm-global.ini')
  mkdirSync(host, { recursive: true })
  writeFileSync(userConfig, '')
  writeFileSync(globalConfig, '')
  const npmEnv = Object.fromEntries(
    Object.entries(process.env).filter(([name]) => !name.toLowerCase().startsWith('npm_config_')),
  )
  const pathKey = Object.keys(npmEnv).find((name) => name.toLowerCase() === 'path') ?? 'Path'
  npmEnv[pathKey] = `${dirname(node)}${delimiter}${npmEnv[pathKey] ?? ''}`
  npmEnv.NPM_CONFIG_USERCONFIG = userConfig
  npmEnv.NPM_CONFIG_GLOBALCONFIG = globalConfig
  npmEnv.NPM_CONFIG_REGISTRY = NPM_REGISTRY
  try {
    run(node, [
      npmCli,
      'install',
      '--prefix',
      host,
      `--registry=${NPM_REGISTRY}`,
      '--@deepseek-ai:registry=https://registry.npmjs.org/',
      '--no-audit',
      '--no-fund',
      `@deepseek-ai/dsh@${DSH_VERSION}`,
    ], { env: npmEnv })
  } finally {
    rmSync(userConfig, { force: true })
    rmSync(globalConfig, { force: true })
  }
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

const staging = mkdtempSync(join(root, '.runtime-stage-'))
const host = join(staging, 'host')
const nodeDir = join(staging, 'node')

try {
  const { node, npmCli, extracted } = installNode(nodeDir, staging)
  const dshBin = installDsh(node, npmCli, host, staging)
  rmSync(extracted, { recursive: true, force: true })
  const pruned = pruneRuntime(host)
  verifyOfficialProfile(node, dshBin)
  const audit = auditRuntime(staging)
  replaceRuntime(staging, runtime)

  console.log(`Pruned ${pruned.removedFiles} files (${(pruned.removedBytes / 1024 / 1024).toFixed(2)} MiB)`)
  console.log(`Runtime ${audit.files} files, ${(audit.bytes / 1024 / 1024).toFixed(2)} MiB`)
  console.log('bundle-host done')
} finally {
  rmSync(staging, { recursive: true, force: true })
}
