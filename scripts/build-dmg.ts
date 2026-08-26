import { cp, mkdir, mkdtemp, readdir, rm, symlink } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"

const ROOT = join(import.meta.dir, "..")
const BUNDLE_ROOT = join(ROOT, "src-tauri", "target", "release", "bundle")
const MACOS_DIRECTORY = join(BUNDLE_ROOT, "macos")
const APP_PATH = join(MACOS_DIRECTORY, "Galpi.app")
const DMG_DIRECTORY = join(BUNDLE_ROOT, "dmg")

class DmgBuildError extends Error {
  readonly name = "DmgBuildError"
}

/// The version the app was actually built at, read from the same file the
/// bundler reads, so a release never ships an artifact named after the
/// previous version.
async function bundleVersion(): Promise<string> {
  const config = (await Bun.file(join(ROOT, "src-tauri", "tauri.conf.json")).json()) as {
    version?: unknown
  }
  if (typeof config.version !== "string" || config.version.length === 0) {
    throw new DmgBuildError("tauri.conf.json is missing a version")
  }
  return config.version
}

async function run(command: readonly string[]): Promise<void> {
  const child = Bun.spawn([...command], {
    stdout: "inherit",
    stderr: "inherit",
  })
  const exitCode = await child.exited
  if (exitCode !== 0) {
    throw new DmgBuildError(`${command[0]} exited with code ${exitCode}`)
  }
}

if (!(await Bun.file(join(APP_PATH, "Contents", "Info.plist")).exists())) {
  throw new DmgBuildError(`Galpi.app not found: ${APP_PATH}`)
}

const DMG_PATH = join(DMG_DIRECTORY, `Galpi_${await bundleVersion()}_aarch64.dmg`)

await rm(DMG_DIRECTORY, { recursive: true, force: true })
await rm(join(BUNDLE_ROOT, "share"), { recursive: true, force: true })
for (const entry of await readdir(MACOS_DIRECTORY)) {
  if (entry.startsWith("rw.") && entry.endsWith(".dmg")) {
    await rm(join(MACOS_DIRECTORY, entry), { force: true })
  }
}
await mkdir(DMG_DIRECTORY, { recursive: true })
const staging = await mkdtemp(join(tmpdir(), "galpi-dmg-"))

try {
  await cp(APP_PATH, join(staging, "Galpi.app"), {
    recursive: true,
    force: true,
    verbatimSymlinks: true,
  })
  await symlink("/Applications", join(staging, "Applications"))
  await run([
    "hdiutil",
    "create",
    "-volname",
    "Galpi",
    "-srcfolder",
    staging,
    "-ov",
    "-format",
    "ULMO",
    DMG_PATH,
  ])
} finally {
  await rm(staging, { recursive: true, force: true })
}
