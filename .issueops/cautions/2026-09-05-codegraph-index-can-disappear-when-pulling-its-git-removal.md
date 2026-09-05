---
name: 2026-09-05-codegraph-index-can-disappear-when-pulling-its-git-removal
description: Caution record for a solved false case or recurring risk.
---

# Codegraph index can disappear when pulling its Git removal

- Date: 2026-09-05
- Kind: `caution`
- Source: project-docs-update
- Summary: Pulling commit 3db4c8b removed the previously tracked local Codegraph database; MCP remained available but reported that the project was not indexed.
- Context: On 2026-09-05 at 09:18 KST, the checkout fast-forwarded to 3db4c8b. That commit deletes .codegraph/codegraph.db and adds .codegraph/ to the root .gitignore. The directory still contained WAL, shared-memory, socket and log files, but the main DB was absent. An explicit codegraph_explore query for AppController returned an unindexed-project error, despite the directory existing. The commit message's claim that existing checkouts keep their local index did not hold for this checkout.
- Resolution: Check the actual .codegraph/codegraph.db file and run `codegraph status --json .`; a directory, daemon log or registered MCP tool alone does not prove that an index exists. If the DB is missing and Codegraph is needed, rebuild it locally with `codegraph index .`, then verify status and a symbol query. Keep .codegraph/ ignored; do not commit the regenerated DB. At 09:22 KST the DB existed again and status reported initialized=true, 113 files, 1760 nodes, no pending changes and reindexRecommended=false. Unknown / not confirmed: the command that recreated it and successful MCP querying after recreation were not observed in this session; verify a query in a subsequent session before claiming MCP recovery.
- Evidence:
  - git show 3db4c8b -- .gitignore .codegraph/.gitignore; the same commit deletes the tracked codegraph.db.
  - git reflog: 2026-09-05 09:18:04 +0900 pull --no-edit --ff-only origin refs/heads/main: Fast-forward.
  - ls -lah .codegraph before recovery: codegraph.db absent; codegraph.db-wal and codegraph.db-shm present.
  - codegraph_explore(projectPath=/Users/m16khb/Workspace/galpi, query=AppController): project is not indexed.
  - codegraph index --help: rebuilds the full index from scratch.
  - codegraph status --json /Users/m16khb/Workspace/galpi: initialized=true; lastIndexed=2026-09-05T00:21:46.143Z; fileCount=113; nodeCount=1760; pendingChanges all zero.
  - git check-ignore .codegraph/codegraph.db: .codegraph/codegraph.db.
  - User requested project-docs-update after the Codegraph failure diagnosis.
