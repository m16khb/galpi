---
name: 2026-08-23-dom-visibility-is-not-pixel-visibility-grid-row-collapse-hid
description: Caution record for a solved false case or recurring risk.
---

# DOM visibility is not pixel visibility: grid-row collapse hid the error banner while text checks passed

- Date: 2026-08-23
- Kind: `caution`
- Source: project-bootstrap enrichment pass
- Summary: The #app-error banner passed textContent/hidden assertions across two QA engagements while a grid-template-rows mismatch (3-row template, 4 children) plus overlay by .workspace-body made the text invisible to users.
- Resolution: a05ea3f reserved a grid row for the banner. Verify user-visible text claims with geometry probes (computed gridTemplateRows, elementsFromPoint at the element center, range bounding box vs opaque siblings), never textContent/hidden alone; extend the colocated DOM tests for view-level regressions.
- Evidence:
  - git a05ea3f fix(ui): reserve a grid row so the error banner text stays visible
  - src/styles.test.ts and src/ui/*.dom.test.ts colocated view tests
  - QA engagement note 2026-08-20 (galpi session)
