#!/usr/bin/env node
// Start the packaged official DSH Web sidecar with a clean temporary home,
// verify the real HTTP page, and tear down the complete process tree.
import { execFileSync, spawn } from 'node:child_process'
import { cpSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { DEFAULT_BUNDLES, validateDefaultProfile } from './default-profile.mjs'

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const root = join(scriptsDir, '..')
const node = join(root, 'runtime', 'node', 'node.exe')
const dshBin = join(root, 'runtime', 'host', 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
const seedProfile = join(root, 'runtime', 'profile-seed', 'profiles', 'web')
const home = mkdtempSync(join(tmpdir(), 'dsh-studio-smoke-'))
const useSeed = process.env.DSH_SMOKE_USE_SEED === '1'

let child

const killTree = () => {
  if (!child?.pid) return
  try {
    execFileSync('taskkill', ['/F', '/T', '/PID', String(child.pid)], { stdio: 'ignore' })
  } catch {
    // The process may already have exited.
  }
}

const waitForUrl = (timeoutMs) => new Promise((resolve, reject) => {
  let output = ''
  const timer = setTimeout(() => reject(new Error(`sidecar was not ready after ${timeoutMs} ms`)), timeoutMs)

  const consume = (chunk) => {
    output += chunk
    process.stdout.write(chunk)
    const match = output.match(/http:\/\/127\.0\.0\.1:(\d+)/)
    if (!match) return
    clearTimeout(timer)
    resolve(match[0])
  }

  child.stdout.on('data', consume)
  child.stderr.on('data', (chunk) => {
    output += chunk
    process.stderr.write(chunk)
  })
  child.once('exit', (code) => {
    clearTimeout(timer)
    reject(new Error(`sidecar exited before readiness with code ${code}`))
  })
})

const main = async () => {
  try {
    if (useSeed) {
      validateDefaultProfile(seedProfile)
      mkdirSync(join(home, 'profiles'), { recursive: true })
      cpSync(seedProfile, join(home, 'profiles', 'web'), {
        recursive: true,
        errorOnExist: true,
      })
    }

    const config = execFileSync(node, [dshBin, 'web', '--dump-config'], {
      encoding: 'utf8',
      env: { ...process.env, DSH_HOME: home },
    })
    if (config.includes('@linxin666')) throw new Error('enhanced Web UI leaked into the official profile')
    if (useSeed) {
      for (const bundle of DEFAULT_BUNDLES) {
        if (!config.includes(bundle)) throw new Error(`seeded bundle missing from config: ${bundle}`)
      }
    }

    child = spawn(node, [dshBin, 'web', '--host', '127.0.0.1', '--port', '0'], {
      env: { ...process.env, DSH_HOME: home },
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    })

    const url = await waitForUrl(90_000)
    const response = await fetch(`${url}/`)
    console.log(`\nHTTP ${response.status} at ${url}/`)
    if (response.status < 200 || response.status >= 400) {
      throw new Error(`unexpected UI status ${response.status}`)
    }
    console.log(`SMOKE OK: ${useSeed ? 'seeded' : 'official'} DSH Web`)
  } finally {
    killTree()
    rmSync(home, { recursive: true, force: true })
  }
}

main().catch((error) => {
  console.error(`\nSMOKE FAIL: ${error.message}`)
  process.exitCode = 1
})
