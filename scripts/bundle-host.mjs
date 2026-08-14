#!/usr/bin/env node
// 打包 runtime：下载 Node、npm 安装 @deepseek-ai/dsh、安装 dsh-web-ui 插件。
//
// 运行环境要求（仅构建机，终端用户不需要）：curl、unzip（Git Bash 自带）、npm、pnpm。
//
// profile 布局（实测 2026-08-14，DSH_HOME=runtime/home）：
//   runtime/home/profiles/web/                    ← web profile 目录
//   runtime/home/profiles/web/package.json        ← dependencies + dsh.profile.bundles（4 层）
//   runtime/home/profiles/web/node_modules/       ← pnpm 安装的插件实际落点
//   runtime/home/profiles/web/pnpm-workspace.yaml ← initProfile 生成，packages: ['.']
//
// 关键：profile 自带 pnpm-workspace.yaml（packages: ['.']，即 profile 自身是 workspace 根），
// 因此 pnpm add 必须带 -w，否则报 ERR_PNPM_ADDING_TO_ROOT。
import { execFileSync } from 'node:child_process'
import { mkdirSync, existsSync, rmSync, renameSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')
const runtime = join(root, 'runtime')
const NODE_VERSION = '24.13.0'
const DSH_VERSION = '0.1.0-rc.6'
const PLUGIN_VERSION = '0.1.11'

function run(cmd, args, opts = {}) {
  console.log('>', cmd, args.join(' '))
  execFileSync(cmd, args, { stdio: 'inherit', ...opts })
}

// 1. 下载并解包 Node 运行时（win-x64）
if (!existsSync(join(runtime, 'node', 'node.exe'))) {
  mkdirSync(join(runtime, 'node'), { recursive: true })
  const zip = join(runtime, 'node.zip')
  run('curl', ['-sSL', '-o', zip, `https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip`])
  run('unzip', ['-o', '-q', zip, '-d', join(runtime, 'node')])
  const inner = join(runtime, 'node', `node-v${NODE_VERSION}-win-x64`)
  renameSync(join(inner, 'node.exe'), join(runtime, 'node', 'node.exe'))
  rmSync(inner, { recursive: true, force: true })
  rmSync(zip, { force: true })
}

const node = join(runtime, 'node', 'node.exe')

// 2. npm 安装 @deepseek-ai/dsh 到 runtime/host
if (!existsSync(join(runtime, 'host', 'node_modules', '@deepseek-ai', 'dsh'))) {
  mkdirSync(join(runtime, 'host'), { recursive: true })
  run('npm', ['install', '--prefix', join(runtime, 'host'), `@deepseek-ai/dsh@${DSH_VERSION}`])
}

const dshBin = join(runtime, 'host', 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')

// 3. 在受控 DSH_HOME 下安装插件（需要本机 pnpm；profile 是 workspace 根，须加 -w）
const home = join(runtime, 'home')
if (!existsSync(join(home, 'profiles'))) {
  mkdirSync(home, { recursive: true })
  run(
    node,
    [dshBin, 'plugin', '--profile', 'web', 'add', '-w',
      `@linxin666/dsh-web-ui-all@${PLUGIN_VERSION}`,
      `@linxin666/dsh-skins@${PLUGIN_VERSION}`],
    { env: { ...process.env, DSH_HOME: home } },
  )
}

// 4. 校验：dump-config 应包含插件层
run(node, [dshBin, 'web', '--dump-config'], { env: { ...process.env, DSH_HOME: home } })
console.log('bundle-host done')
