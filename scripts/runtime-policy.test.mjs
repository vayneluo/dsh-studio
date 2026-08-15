import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import test from 'node:test'

import { auditRuntime, pruneRuntime } from './runtime-policy.mjs'

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
  for (const path of [files.pdb, files.map, files.dts, files.dmts, files.dcts, files.armPty, files.darwinPty, files.source]) {
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
