import type { Dirent } from "node:fs"
import { readdir, readFile } from "node:fs/promises"
import { join } from "node:path"

const RUST_ROOT = join(import.meta.dir, "..", "src-tauri", "src")

interface Fence {
  readonly root: string
  readonly forbidden: readonly string[]
}

const fences: readonly Fence[] = [
  {
    root: join(RUST_ROOT, "domain"),
    forbidden: ["crate::application", "crate::adapters", "crate::composition", "tauri::"],
  },
  {
    root: join(RUST_ROOT, "application"),
    forbidden: ["crate::adapters", "crate::composition", "tauri::"],
  },
  {
    root: join(RUST_ROOT, "adapters", "inbound"),
    forbidden: ["adapters::outbound", "crate::composition"],
  },
  {
    root: join(RUST_ROOT, "adapters", "outbound"),
    forbidden: ["adapters::inbound", "crate::composition"],
  },
]

class ArchitectureError extends Error {
  readonly name = "ArchitectureError"
}

async function rustFiles(root: string): Promise<readonly string[]> {
  const entries: readonly Dirent[] = await readdir(root, { withFileTypes: true })
  const nested = await Promise.all(
    entries.map(async (entry): Promise<readonly string[]> => {
      const path = join(root, entry.name)
      if (entry.isDirectory()) {
        return rustFiles(path)
      }
      return entry.isFile() && path.endsWith(".rs") ? [path] : []
    }),
  )
  return nested.flat()
}

async function checkFence(fence: Fence): Promise<readonly string[]> {
  const violations: string[] = []
  for (const path of await rustFiles(fence.root)) {
    const source = await readFile(path, "utf8")
    for (const forbidden of fence.forbidden) {
      if (source.includes(forbidden)) {
        violations.push(`${path}: forbidden dependency ${forbidden}`)
      }
    }
  }
  return violations
}

async function checkFrameworkLocality(): Promise<readonly string[]> {
  const violations: string[] = []
  const inbound = join(RUST_ROOT, "adapters", "inbound", "tauri.rs")
  const composition = join(RUST_ROOT, "composition.rs")
  const processAdapter = join(RUST_ROOT, "adapters", "outbound", "process.rs")
  const processAdapterDirectory = join(RUST_ROOT, "adapters", "outbound", "process")
  for (const path of await rustFiles(RUST_ROOT)) {
    const source = await readFile(path, "utf8")
    if (source.includes("#[tauri::command]") && path !== inbound) {
      violations.push(`${path}: Tauri command belongs in the inbound adapter`)
    }
    if (
      (source.includes("generate_handler!") ||
        source.includes(".manage(") ||
        source.includes(".plugin(")) &&
      path !== composition
    ) {
      violations.push(`${path}: Tauri composition belongs in composition.rs`)
    }
    const usesNix = /(^|[\s({])nix::/m.test(source)
    if (
      (source.includes("tokio::process") || usesNix) &&
      path !== processAdapter &&
      !path.startsWith(`${processAdapterDirectory}/`)
    ) {
      violations.push(`${path}: process primitives belong in the process adapter`)
    }
  }
  return violations
}

const violations = [
  ...(await Promise.all(fences.map(checkFence))).flat(),
  ...(await checkFrameworkLocality()),
]
if (violations.length > 0) {
  throw new ArchitectureError(violations.join("\n"))
}
