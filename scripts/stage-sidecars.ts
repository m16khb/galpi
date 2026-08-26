import { createHash } from "node:crypto"
import { chmod, cp, mkdir, mkdtemp, rm } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"

const UV_VERSION = "0.12.5"
const TARGET = "aarch64-apple-darwin"
const ARCHIVE_SHA256 = "5bb0e5fe008a773c3dbcb97ff79cd89e1241464fe9d2f986d52ad8f1b037bd62"
const BINARY_SHA256 = "ad3564874e19defa0debefcf48e8381ac1d087c584190c1323c247bd351dd25f"
const BINARY_DIR = join(import.meta.dir, "..", "src-tauri", "binaries")
const BINARY_PATH = join(BINARY_DIR, `uv-${TARGET}`)
const WORKER_SOURCE = join(import.meta.dir, "..", "worker")
const WORKER_DESTINATION = join(import.meta.dir, "..", "src-tauri", "resources", "worker")

class SidecarStageError extends Error {
  readonly name = "SidecarStageError"
}

async function run(command: readonly string[]): Promise<void> {
  const child = Bun.spawn([...command], {
    stdout: "inherit",
    stderr: "inherit",
  })
  const exitCode = await child.exited
  if (exitCode !== 0) {
    throw new SidecarStageError(`${command[0]} exited with code ${exitCode}`)
  }
}

async function stageUv(): Promise<void> {
  if (await Bun.file(BINARY_PATH).exists()) {
    const checksum = createHash("sha256")
      .update(new Uint8Array(await Bun.file(BINARY_PATH).arrayBuffer()))
      .digest("hex")
    if (checksum === BINARY_SHA256) return
    await rm(BINARY_PATH)
  }

  await mkdir(BINARY_DIR, { recursive: true })
  const workDir = await mkdtemp(join(tmpdir(), "galpi-uv-"))
  const archivePath = join(workDir, `uv-${TARGET}.tar.gz`)
  const releaseUrl = `https://github.com/astral-sh/uv/releases/download/${UV_VERSION}/uv-${TARGET}.tar.gz`

  await run(["curl", "-fsSL", releaseUrl, "-o", archivePath])
  const archive = new Uint8Array(await Bun.file(archivePath).arrayBuffer())
  const checksum = createHash("sha256").update(archive).digest("hex")
  if (checksum !== ARCHIVE_SHA256) {
    throw new SidecarStageError(`uv archive checksum mismatch: ${checksum}`)
  }

  await run(["tar", "-xzf", archivePath, "-C", workDir])
  await Bun.write(BINARY_PATH, Bun.file(join(workDir, `uv-${TARGET}`, "uv")))
  await chmod(BINARY_PATH, 0o755)
}

async function stageWorker(): Promise<void> {
  await rm(WORKER_DESTINATION, { recursive: true, force: true })
  await mkdir(WORKER_DESTINATION, { recursive: true })
  await cp(join(WORKER_SOURCE, "galpi_worker"), join(WORKER_DESTINATION, "galpi_worker"), {
    recursive: true,
    force: true,
    filter: (source) => !source.includes("__pycache__") && !source.endsWith(".pyc"),
  })
  // The locks are what the installer actually reads; the loose requirements
  // files travel with them so the pins stay readable next to their source.
  const requirements = [
    "requirements.txt",
    "requirements.lock",
    "requirements-qwen3.txt",
    "requirements-qwen3.lock",
  ]
  for (const file of requirements) {
    await cp(join(WORKER_SOURCE, file), join(WORKER_DESTINATION, file))
  }
}

await stageUv()
await stageWorker()
