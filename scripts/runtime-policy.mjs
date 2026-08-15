import { existsSync, lstatSync, readdirSync, renameSync, rmSync, statSync } from 'node:fs'
import { basename, join, normalize } from 'node:path'

export const DEFAULT_PROFILE_BUDGET_BYTES = 35 * 1024 * 1024
export const DEFAULT_RUNTIME_BUDGET_BYTES = 260 * 1024 * 1024
export const DEFAULT_INSTALLER_BUDGET_BYTES = 70 * 1024 * 1024

const removableTypePattern = /(?:\.pdb|\.map|\.d\.(?:ts|mts|cts)|\.(?:ts|tsx|mts|cts|c|cc|cpp|h|hpp|gyp|gypi|dylib)|\.so(?:\.\d+)*)$/i
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

const findReparsePaths = (root) => {
  const reparsePaths = []
  const directories = [root]

  while (directories.length > 0) {
    const directory = directories.pop()
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name)
      const stat = lstatSync(path)
      if (stat.isSymbolicLink()) {
        reparsePaths.push(path)
        continue
      }
      if (stat.isDirectory()) directories.push(path)
    }
  }

  return reparsePaths
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

const findPrebuildDirectories = (root) => {
  const matches = []
  const directories = [root]

  while (directories.length > 0) {
    const directory = directories.pop()
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (!entry.isDirectory() || entry.isSymbolicLink()) continue
      const path = join(directory, entry.name)
      if (entry.name === 'prebuilds') matches.push(path)
      else directories.push(path)
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

  }

  for (const prebuilds of findPrebuildDirectories(hostDir)) {
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

export const replaceRuntime = (stagingDir, runtimeDir) => {
  const backupDir = `${runtimeDir}.backup`

  // Recover a swap interrupted between moving the previous runtime aside and
  // publishing the freshly built staging tree.
  if (existsSync(backupDir)) {
    if (existsSync(runtimeDir)) {
      rmSync(backupDir, { recursive: true, force: true })
    } else {
      renameSync(backupDir, runtimeDir)
    }
  }

  let previousRuntimeMoved = false
  try {
    if (existsSync(runtimeDir)) {
      renameSync(runtimeDir, backupDir)
      previousRuntimeMoved = true
    }
    renameSync(stagingDir, runtimeDir)
  } catch (error) {
    if (!existsSync(runtimeDir) && previousRuntimeMoved && existsSync(backupDir)) {
      renameSync(backupDir, runtimeDir)
    }
    throw error
  }

  rmSync(backupDir, { recursive: true, force: true })
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

export const auditPortableTree = (
  root,
  { maxBytes = DEFAULT_PROFILE_BUDGET_BYTES } = {},
) => {
  const reparsePaths = findReparsePaths(root)
  if (reparsePaths.length > 0) {
    throw new Error(`Profile contains a reparse point, symbolic link, or junction: ${reparsePaths[0]}`)
  }

  const physicalFiles = walkPhysicalFiles(root)
  const forbiddenPaths = physicalFiles
    .map((file) => file.path)
    .filter((path) => normalize(path).includes(normalize('@linxin666')))
  if (forbiddenPaths.length > 0) {
    throw new Error(`Forbidden profile path: ${forbiddenPaths[0]}`)
  }
  const bytes = physicalFiles.reduce((total, file) => total + file.bytes, 0)
  if (bytes > maxBytes) {
    throw new Error(`Profile budget exceeded: ${bytes} bytes > ${maxBytes} bytes`)
  }

  return { bytes, files: physicalFiles.length, forbiddenPaths, reparsePaths }
}

export const auditInstaller = (
  path,
  { maxBytes = DEFAULT_INSTALLER_BUDGET_BYTES } = {},
) => {
  const bytes = statSync(path).size
  if (bytes > maxBytes) {
    throw new Error(`Installer budget exceeded: ${bytes} bytes > ${maxBytes} bytes`)
  }
  return { bytes, path }
}
