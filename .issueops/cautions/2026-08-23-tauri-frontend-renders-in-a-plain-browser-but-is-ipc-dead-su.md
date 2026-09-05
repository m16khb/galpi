---
name: 2026-08-23-tauri-frontend-renders-in-a-plain-browser-but-is-ipc-dead-su
description: Caution record for a solved false case or recurring risk.
---

# Tauri frontend renders in a plain browser but is IPC-dead; subscribe before invoke

- Date: 2026-08-23
- Kind: `caution`
- Source: project-bootstrap enrichment pass
- Summary: The Vite frontend fully renders its DOM shell in a plain browser while every invoke/listen rejects (transformCallback TypeError), so IPC-bound behavior looks silently broken; separately, awaiting Tauri listen before bind() in controller.start() once bricked the UI silently (BUG-01, 2026-08-20).
- Resolution: Bind-first controller startup plus a persistent app-level error banner. In plain-browser QA, classify IPC-bound behavior as Not Run/Blocked instead of failed UI logic. Standing rule: subscribe to Tauri events before invoking the operation that emits them.
- Evidence:
  - AGENTS.md CONVENTIONS: subscribe to Tauri events before invoking the operation that emits them
  - git ba634d9 fix(ui): surface failures from direct IPC actions
  - git 2576418 fix(ui): announce failures once with user-facing copy
  - QA engagement note 2026-08-20 (galpi session)
