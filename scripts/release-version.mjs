import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const versionPattern = '(0|[1-9]\\d*)\\.(0|[1-9]\\d*)\\.(0|[1-9]\\d*)'
const paths = {
  packageJson: path.join(root, 'package.json'),
  tauriConfig: path.join(root, 'src-tauri/tauri.conf.json'),
  cargoManifest: path.join(root, 'src-tauri/Cargo.toml'),
  cargoLock: path.join(root, 'src-tauri/Cargo.lock'),
}

function normalizeVersion(input) {
  const version = input?.replace(/^v/, '')
  if (!new RegExp(`^${versionPattern}$`).test(version ?? '')) {
    throw new Error(`版本号必须是 x.y.z 格式，收到：${input ?? ''}`)
  }
  return version
}

function replaceFirst(text, pattern, replacement, label) {
  if (!pattern.test(text)) throw new Error(`无法找到 ${label} 的版本字段`)
  return text.replace(pattern, replacement)
}

function setCargoPackageVersion(text, version) {
  const start = text.indexOf('[package]')
  const nextSection = text.indexOf('\n[', start + '[package]'.length)
  if (start < 0) throw new Error('Cargo.toml 缺少 [package]')
  const end = nextSection < 0 ? text.length : nextSection
  const section = text.slice(start, end)
  const updated = replaceFirst(
    section,
    /^(version\s*=\s*")[^"]+("\s*)$/m,
    `$1${version}$2`,
    'Cargo.toml [package]',
  )
  return text.slice(0, start) + updated + text.slice(end)
}

function cargoPackageVersion(text) {
  const match = text.match(/\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)
  if (!match) throw new Error('Cargo.toml 缺少 [package] 版本字段')
  return match[1]
}

function lockPackageVersion(text) {
  const match = text.match(/\[\[package\]\]\r?\nname = "tinytodo"\r?\nversion = "([^"]+)"/)
  if (!match) throw new Error('Cargo.lock 缺少 tinytodo crate 记录')
  return match[1]
}

async function readReleaseFiles() {
  const [packageText, tauriText, cargoManifest, cargoLock] = await Promise.all([
    readFile(paths.packageJson, 'utf8'),
    readFile(paths.tauriConfig, 'utf8'),
    readFile(paths.cargoManifest, 'utf8'),
    readFile(paths.cargoLock, 'utf8'),
  ])
  return {
    packageJson: JSON.parse(packageText),
    tauriConfig: JSON.parse(tauriText),
    cargoManifest,
    cargoLock,
  }
}

function assertVersions(files, expected) {
  const versions = {
    'package.json': files.packageJson.version,
    'src-tauri/tauri.conf.json': files.tauriConfig.version,
    'src-tauri/Cargo.toml': cargoPackageVersion(files.cargoManifest),
    'src-tauri/Cargo.lock': lockPackageVersion(files.cargoLock),
  }
  const mismatches = Object.entries(versions).filter(([, value]) => value !== expected)
  if (mismatches.length) {
    throw new Error(`版本不一致：${mismatches.map(([file, value]) => `${file}=${value}`).join(', ')}`)
  }
}

async function prepare(input) {
  const version = normalizeVersion(input)
  const files = await readReleaseFiles()
  files.packageJson.version = version
  files.tauriConfig.version = version
  files.cargoManifest = setCargoPackageVersion(files.cargoManifest, version)
  files.cargoLock = replaceFirst(
    files.cargoLock,
    /(\[\[package\]\]\r?\nname = "tinytodo"\r?\nversion = ")[^"]+(\")/,
    `$1${version}$2`,
    'Cargo.lock tinytodo package',
  )
  await Promise.all([
    writeFile(paths.packageJson, `${JSON.stringify(files.packageJson, null, 2)}\n`),
    writeFile(paths.tauriConfig, `${JSON.stringify(files.tauriConfig, null, 2)}\n`),
    writeFile(paths.cargoManifest, files.cargoManifest),
    writeFile(paths.cargoLock, files.cargoLock),
  ])
  console.log(`已将 TinyTodo 版本更新为 ${version}。提交这些版本文件后，创建并推送标签 v${version}。`)
}

async function verify(input) {
  if (!new RegExp(`^v${versionPattern}$`).test(input ?? '')) {
    throw new Error(`发布标签必须是 vX.Y.Z 格式，收到：${input ?? ''}`)
  }
  const expected = normalizeVersion(input)
  assertVersions(await readReleaseFiles(), expected)
  console.log(`发布标签 ${input} 与所有项目版本一致。`)
}

const [commandOrVersion, input] = process.argv.slice(2)
if (commandOrVersion === '--verify') {
  await verify(input)
} else if (!commandOrVersion) {
  throw new Error('用法：pnpm release:prepare <version>，例如 pnpm release:prepare 0.2.0')
} else {
  await prepare(commandOrVersion)
}
