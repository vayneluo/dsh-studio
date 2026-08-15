import { lstatSync, readdirSync, rmSync } from 'node:fs'
import { basename, join, normalize } from 'node:path'

export const DEFAULT_RUNTIME_BUDGET_BYTES = 230 * 1024 * 1024

const removableTypePattern = /(?:\.pdb|\.map|\.d\.(?:ts|mts|cts))$/i
const nodePtySourceDirectories = new Set([
  'build',
  'deps',
  'scripts',
  'src',
  'third_party',
  'typings',
])

const walkPhysicalFiles = (root) => {
  const files = []
  const directories = [root]

  while (directories.length > 0) {
    const directory = directories.pop()
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name)
      const stat = lstatSync(path)
      if (stat.isSymbolicLink()) continue
      if (stat.isDirectory()) {
        directories.push(path)
        continue
      }
      if (stat.isFile()) files.push({ path, bytes: stat.size })
    }
  }

  return files
}

const findNodePtyDirectories = (root) => {
  const matches = []
  const directories = [root]

  while (directories.length > 0) {
    const directory = directories.pop()
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (!entry.isDirectory() || entry.isSymbolicLink()) continue
      const path = join(directory, entry.name)
      if (entry.name === 'node-pty' && basename(directory) === 'node_modules') {
        matches.push(path)
        continue
      }
      directories.push(path)
    }
  }

  return matches
}

const removeDirectory = (path) => {
  const files = walkPhysicalFiles(path)
  const bytes = files.reduce((total, file) => total + file.bytes, 0)
  rmSync(path, { recursive: true, force: true })
  return { removedFiles: files.length, removedBytes: bytes }
}

export const pruneRuntime = (hostDir) => {
  let removedFiles = 0
  let removedBytes = 0

  for (const nodePtyDir of findNodePtyDirectories(hostDir)) {
    for (const directory of nodePtySourceDirectories) {
      const path = join(nodePtyDir, directory)
      try {
        const result = removeDirectory(path)
        removedFiles += result.removedFiles
        removedBytes += result.removedBytes
      } catch (error) {
        if (error?.code !== 'ENOENT') throw error
      }
    }

    const prebuilds = join(nodePtyDir, 'prebuilds')
    try {
      for (const entry of readdirSync(prebuilds, { withFileTypes: true })) {
        if (!entry.isDirectory() || entry.name === 'win32-x64') continue
        const result = removeDirectory(join(prebuilds, entry.name))
        removedFiles += result.removedFiles
        removedBytes += result.removedBytes
      }
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error
    }
  }

  for (const file of walkPhysicalFiles(hostDir)) {
    if (!removableTypePattern.test(file.path)) continue
    rmSync(file.path, { force: true })
    removedFiles += 1
    removedBytes += file.bytes
  }

  return { removedFiles, removedBytes }
}

export const auditRuntime = (
  runtimeDir,
  { maxBytes = DEFAULT_RUNTIME_BUDGET_BYTES } = {},
) => {
  const physicalFiles = walkPhysicalFiles(runtimeDir)
  const paths = physicalFiles.map((file) => file.path)
  const forbiddenPaths = paths.filter((path) => normalize(path).includes(`${normalize('@linxin666')}`))
  const bytes = physicalFiles.reduce((total, file) => total + file.bytes, 0)

  if (forbiddenPaths.length > 0) {
    throw new Error(`Forbidden runtime path: ${forbiddenPaths[0]}`)
  }
  if (bytes > maxBytes) {
    throw new Error(`Runtime budget exceeded: ${bytes} bytes > ${maxBytes} bytes`)
  }

  return {
    bytes,
    files: physicalFiles.length,
    forbiddenPaths,
    paths,
  }
}
