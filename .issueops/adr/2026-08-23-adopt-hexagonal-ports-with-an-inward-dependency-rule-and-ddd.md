---
name: 2026-08-23-adopt-hexagonal-ports-with-an-inward-dependency-rule-and-ddd
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Adopt hexagonal ports with an inward dependency rule and DDD tactical patterns

- Date: 2026-08-23
- Kind: `adr`
- Source: project-bootstrap enrichment pass
- Summary: One dependency rule across TS/Rust/Python keeps frameworks in adapters and composition roots; codified in docs/ARCHITECTURE.md and enforced by check-architecture fences.
- Context: Three runtimes (TS WebView, Rust Tauri host, Python WhisperX sidecar) needed one consistent dependency rule; DIP violations found 2026-08-21 (BackendPort defined in adapters, roster value objects in application/model.rs) were fixed and codified.
- Decision: Dependencies point inward (domain) in all three runtimes; ports are owned by the consuming inner layer (application/ports.rs traits, src/domain/backend.ts BackendPort) and implemented by adapters; frameworks (Tauri, Zod, CPAL, tokio::process) live only in adapters and composition roots; DDD tactical value objects and the Artifacts aggregate live in domain layers.
- Consequences: ["Boundary questions resolve in docs/ARCHITECTURE.md first", "Dependency fences enforced by scripts/check-architecture.ts inside bun run check", "Worker protocol, Rust parser, frontend event schema, and job reducer form one change set"]
- Evidence:
  - docs/ARCHITECTURE.md §1 dependency rule, §2 port ownership, §4 SOLID table, §6 violations and deliberate non-refactors
  - git 8eca865 docs(architecture): document DDD/hexagonal/clean/OOP/SOLID mapping
  - git 6239612 test(architecture): enforce frontend dependency fences
  - git 73cb73e refactor(frontend): own the backend port contract in domain
  - git 4476db0 refactor(rust): move roster value objects into the domain
- Alternatives / rejected options:
  - Splitting DesktopAdapter's 4 implemented ports into 4 adapter structs (boilerplate in composition root, single consumer)
  - Hiding JobRegistry behind a repository port (in-memory state; abstraction cost before a second storage exists)
  - Splitting the unified TS BackendPort per ISP (single consumer: AppController)
  - Enforcing layer directories in the Python worker CLI sidecar (pure modules testable without ML stack are the real boundary)
