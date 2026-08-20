# Galpi Design System

## 0. Research Log

- Embedded refs: shortlisted ElevenLabs, Linear, and Notion; picked operational `taste-skill` + ElevenLabs because an audio workstation benefits from quiet typography, waveform cues, and tactile but restrained controls.
- Lazyweb: searched `audio transcription desktop app` and `meeting recorder transcription speakers`; viewed Descript and VOMO screens. Kept Descript's task-first canvas and compact utility rail, while rejecting its editing-tool density for Galpi's simpler prepare-and-run workflow.
- StyleGallery: adopted `scroll-body-shell` for one bounded main scroll owner and `supporting-pane` for setup/progress context beside the primary task.
- Interaction reference: consulted beui.dev `button` source; retained explicit idle/loading/success/error labels, `aria-live`, press feedback, and reduced-motion behavior without importing its React implementation.
- Imagen drafts: skipped after two `generate_image` calls returned HTTP 404; no generated image is used as a visual contract.

## 1. Atmosphere & Identity

Galpi is a calm local audio workbench: technically precise, quiet during long work, and explicit about what is installed, downloaded, and running. Its signature is a horizontal waveform rule that fills by pipeline phase, making invisible model work legible without pretending to estimate time remaining.

## 2. Color

### Palette

| Role | Token | Light | Usage |
|---|---|---:|---|
| Surface/primary | `--surface-primary` | `#f7f6f2` | Window canvas |
| Surface/secondary | `--surface-secondary` | `#efede7` | Setup rail, grouped controls |
| Surface/elevated | `--surface-elevated` | `#fffefa` | Task and result surfaces |
| Surface/inverse | `--surface-inverse` | `#20201e` | Log panel |
| Text/primary | `--text-primary` | `#24231f` | Headings and body |
| Text/secondary | `--text-secondary` | `#666159` | Supporting copy |
| Text/inverse | `--text-inverse` | `#f7f6f2` | Log panel text |
| Border/default | `--border-default` | `#d8d4ca` | Inputs and dividers |
| Border/subtle | `--border-subtle` | `#e8e4da` | Surface separation |
| Accent/primary | `--accent-primary` | `#b85c38` | Primary action and current phase |
| Accent/hover | `--accent-hover` | `#98482c` | Primary action hover |
| Accent/text | `--accent-text` | `#98482c` | Small accent labels and text links (eyebrow, section index, phase label, text buttons) |
| Status/success | `--status-success` | `#3f7356` | Ready and completed |
| Status/warning | `--status-warning` | `#8a5e1c` | Setup attention |
| Status/error | `--status-error` | `#a63f3f` | Failures |
| Focus | `--focus-ring` | `#246b9b` | Keyboard focus only |

### Rules

- Accent marks an action or current pipeline state; it is never decorative.
- Surfaces use warm tonal shifts. New colors must be added here first.
- Status colors always pair with text or a state label, never color alone.

## 3. Typography

### Scale

| Level | Size | Weight | Line Height | Tracking | Usage |
|---|---:|---:|---:|---:|---|
| Display | `2.25rem` | 400 | 1.12 | `-0.025em` | Current task heading |
| H1 | `1.75rem` | 500 | 1.2 | `-0.02em` | Page heading |
| H2 | `1.25rem` | 600 | 1.3 | `-0.01em` | Panel heading |
| H3 | `1rem` | 600 | 1.4 | 0 | Control group heading |
| Body/lg | `1rem` | 400 | 1.6 | `0.01em` | Introductory copy |
| Body | `0.9375rem` | 400 | 1.55 | `0.01em` | Default UI |
| Body/sm | `0.8125rem` | 400 | 1.5 | `0.015em` | Supporting text |
| Caption | `0.75rem` | 600 | 1.4 | `0.04em` | Status labels |
| Mono | `0.75rem` | 400 | 1.65 | 0 | Logs and paths |

### Font Stack

- Primary: `"Avenir Next", "Pretendard", -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", sans-serif`
- Mono: `"SFMono-Regular", "JetBrains Mono", Menlo, monospace`

## 4. Spacing & Layout

### Base Unit

All spacing derives from 4px.

| Token | Value | Usage |
|---|---:|---|
| `--space-1` | 4px | Tight icon/label spacing |
| `--space-2` | 8px | Compact inline groups |
| `--space-3` | 12px | Input interior spacing |
| `--space-4` | 16px | Standard grouping |
| `--space-5` | 20px | Panel interior spacing |
| `--space-6` | 24px | Primary panel padding |
| `--space-8` | 32px | Section separation |
| `--space-10` | 40px | Major task separation |

### Grid

- Window target: 1240×820, minimum 920×640.
- Shell: fixed 248px setup rail plus fluid work area above 960px; single column below.
- Main work: intrinsic `supporting-pane`, primary task minimum 32rem where space permits.
- The work area owns vertical scrolling; rail and footer remain fixed.

### Rules

- Use `100dvb`, `minmax(0, 1fr)`, and `min-block-size: 0` for the bounded shell.
- At 375px equivalent width, controls stack into one readable column with no primary horizontal scroll.
- Paths use `overflow-wrap: anywhere`; long file names truncate only when the full value is available in a title.

## 5. Components

### Step Rail

- **Structure**: three user stages — `01 회의 전사`, `02 전사 결과`, `03 전사 결과 AI 증강` — each with a state label and short explanation. Engine and model preparation are a pre-gate panel (`00 / 준비`), not rail stages; they disappear once the local environment is ready.
- **Stage mapping**: `01` completes when transcription artifacts render; `02` becomes current with the results panel; `03` completes when augmented minutes render. The augment stage hint links to Settings when no assistant key is saved and otherwise waits for a transcription. Augmentation streams progress: the refine phase emits `N자 작성됨` updates on the existing phase-event channel while the provider generates.
- **States**: pending, current, completed, blocked.
- **Accessibility**: `aria-current="step"` on the current item; text accompanies every state color.
- **Motion**: current marker fades and translates no more than 4px; no motion under reduced motion.
- **Layout**: fixed shell rail; never owns scroll.

### Status Button

- **Structure**: one label plus optional progress/status glyph from Phosphor Icons.
- **Variants**: primary, secondary, quiet, destructive.
- **States**: idle, loading, success, error, disabled.

### Participant Chips

- **Structure**: the per-meeting attendee picker renders one toggle chip per roster entry (`이름 · 팀 · 역할`, omitting absent parts); a saved roster is edited in Settings under `참석자 명부` with name, optional team, optional role, optional freeform description (`담당 업무 등 설명`), and comma-separated aliases. A chip's description surfaces as its tooltip; the chip label itself stays compact. An unselected chip renders no mark slot — the check glyph appears only on selection.
- **States**: unselected, selected, disabled; selection state pairs the filled accent with a check glyph and the `N명 선택` counter, never color alone.
- **Behavior**: selecting attendees fills the speaker-count hint (`정확히 N`) and a note says the value was auto-filled; a later manual change is never overridden. An empty roster shows a hint linking to Settings instead of an empty chip group.
- **Accessibility**: each chip is a real checkbox inside `role="group"`; focus ring follows the global outline token; chips scroll independently above six entries.

### Glossary

- **Structure**: Settings hosts a `단어집` section of `용어` plus optional `뜻/설명` rows; entries persist with assistant settings and apply to every minutes refinement — there is no per-meeting toggle.
- **Behavior**: entries reach the worker as a `<단어집>` prompt block so misheard terms are corrected against the saved spelling; an empty glossary states that no terms are registered.
- **States**: the section header carries a `N개` counter (or `비어 있음`); rows are removed individually with a labeled X button.
- **Accessibility**: `aria-busy` while loading and polite live label updates.
- **Motion**: 100ms press scale, 180ms opacity label swap; instant under reduced motion.

### Field Group

- **Structure**: visible label, control, optional helper, contextual error.
- **States**: default, hover, focus, disabled, error.
- **Accessibility**: labels are never placeholders; error and helper IDs connect with `aria-describedby`.
- **Layout**: stack primitive; related controls use a wrapping cluster.

### Settings Autosave

- **Behavior**: text fields persist when the user commits the edit (blur or Enter); selects and row removals persist immediately. There is no global save button.
- **Concurrency**: while one local write is active, later changes coalesce into one latest-state write instead of racing or disabling the sheet.
- **Feedback**: the polite settings status line moves through `저장 중` → `자동 저장됨`, or an actionable error. Errors preserve the edited values and the next change retries.
- **Destructive actions**: clearing a stored credential remains an explicit labeled action; autosave never turns a destructive clear into an implicit side effect.
- **Accessibility**: persistence feedback uses the existing `role="status"` live region and never steals focus.

### Phase Timeline

- **Structure**: waveform progress rule plus four named phases and live phase message.
- **States**: waiting, active, completed, failed, cancelled.
- **Accessibility**: `role="progressbar"` with phase-local value; no fabricated time estimate.
- **Motion**: waveform fill uses transform only and stops under reduced motion.

### Artifact Row

- **Structure**: artifact kind, canonical path, open and reveal actions.
- **States**: ready, opening, missing, error.
- **Accessibility**: action labels include artifact kind; paths remain selectable.
- **Layout**: cluster that wraps actions before the path overflows.

### Log Disclosure

- **Structure**: native `details` with capped mono output.
- **States**: collapsed by default, expanded, error-highlighted.
- **Accessibility**: raw diagnostics remain copyable; user-facing error summary sits outside the disclosure.
- **Layout**: the log body owns its own bounded scroll only when expanded.

## 6. Motion & Interaction

| Type | Duration | Easing | Usage |
|---|---:|---|---|
| Micro | 100ms | `ease-out` | Press feedback |
| Standard | 180ms | `cubic-bezier(0.16, 1, 0.3, 1)` | State swaps |
| Emphasis | 320ms | `cubic-bezier(0.16, 1, 0.3, 1)` | Phase transition |

- Animate only transform, opacity, and progress clip/scale.
- Subscribe to Tauri events before invoking setup or transcription.
- Running work always exposes a cancel action.
- `prefers-reduced-motion: reduce` removes transforms and keeps instant state changes.

## 7. Depth & Surface

Strategy: mixed tonal shift and whisper-level warm shadows.

| Level | Value | Usage |
|---|---|---|
| Edge | `inset 0 0 0 1px rgb(89 72 51 / 0.08)` | Inputs and compact controls |
| Rest | `0 1px 2px rgb(78 50 23 / 0.05), 0 8px 24px rgb(78 50 23 / 0.04)` | Main surfaces |
| Raised | `0 2px 6px rgb(78 50 23 / 0.07), 0 18px 42px rgb(78 50 23 / 0.07)` | Modal or active result |

- Cards use 14px radius; inputs use 10px; primary buttons are pills.
- No glass blur, outer glow, or pure black shadow.

## 8. Accessibility Constraints & Accepted Debt

### Constraints

- WCAG 2.2 AA: 4.5:1 body text, 3:1 large text and controls.
- Every action is keyboard reachable with a visible focus ring.
- Native dialogs handle file and folder selection.
- Progress and errors are announced without repeatedly reading raw log lines.
- Touch targets are at least 40×40px.
- Long model downloads never rely on color, animation, or elapsed-time guesses.

### Accepted Debt

| Item | Location | Why accepted | Owner / Exit |
|---|---|---|---|
| No dark theme in first release | Whole app | The desktop utility uses one controlled warm-light workspace; a second theme would double initial visual QA without changing task completion. | Add only after a user preference request. |
| macOS ARM64 packaging first | Build pipeline | Current target workstation is Apple Silicon and ML dependencies are platform-heavy. | Add signed Intel/Windows packages with platform-specific QA. |
