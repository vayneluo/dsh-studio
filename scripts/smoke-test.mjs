#!/usr/bin/env node
// 冒烟：以 sidecar 方式起宿主（node <dsh_bin> web），验证就绪 + UI 可达（HTTP 响应），
// 退出后树清理进程，不留残留。
import { spawn, execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')
const node = join(root, 'runtime', 'node', 'node.exe')
const dshBin = join(root, 'runtime', 'host', 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
const home = join(root, 'runtime', 'home')

const child = spawn(node, [dshBin, 'web', '--host', '127.0.0.1', '--port', '0'], {
  env: { ...process.env, DSH_HOME: home },
  stdio: ['ignore', 'pipe', 'pipe'],
})

let out = ''
child.stdout.on('data', (d) => { out += d; process.stdout.write(d) })
child.stderr.on('data', (d) => { out += d; process.stderr.write(d) })

function killTree() {
  try {
    execFileSync('taskkill', ['/F', '/T', '/PID', String(child.pid)], { stdio: 'ignore' })
  } catch { /* 已退出则忽略 */ }
}

function fail(msg) {
  console.error(`\nFAIL: ${msg}`)
  killTree()
  process.exit(1)
}

const deadline = Date.now() + 30_000
const timer = setInterval(() => {
  const m = out.match(/http:\/\/127\.0\.0\.1:(\d+)/)
  if (!m) {
    if (Date.now() > deadline) fail('未在 30s 内就绪')
    return
  }
  clearInterval(timer)
  const url = m[0]
  fetch(`${url}/`).then((res) => {
    console.log(`\nHTTP ${res.status} at ${url}/`)
    if (res.status >= 500) {
      killTree()
      process.exit(1)
    }
    killTree()
    console.log('SMOKE OK')
    process.exit(0)
  }).catch((e) => fail(`UI 不可达: ${e.message}`))
}, 500)
