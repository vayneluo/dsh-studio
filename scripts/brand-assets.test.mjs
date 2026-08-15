import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('../', import.meta.url)

const sha256 = async (path) =>
  createHash('sha256')
    .update(await readFile(new URL(path, root)))
    .digest('hex')

test('supplied brand inputs remain canonical', async () => {
  assert.equal(
    await sha256('ui/logo.png'),
    'd12408a032c5ec00d39f4cc050f3451de04dc90279045a8070dbd2880bdb6b84',
  )
  assert.equal(
    await sha256('ui/brand.png'),
    'e9ed4eeb9a4bcc4433b8977657d7589e3e78ad8ac359c6f86e9d1833f1276594',
  )
})

test('generated app icon no longer uses the legacy D artwork', async () => {
  assert.notEqual(
    await sha256('src-tauri/icons/icon.png'),
    '506bd781a772e4651df1322b0d726ec1a7d7909a000e5236db6dd3c6021d972d',
  )
  assert.notEqual(
    await sha256('src-tauri/icons/icon.ico'),
    'c7b8b8af986fbf16be050bf491e2e703592fd4da85f9e2fe52b8637ffdc0d974',
  )
})
