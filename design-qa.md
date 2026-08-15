# GPUI parity design QA

## Comparison target

- Source visual and behavior oracle: the merged Electron/React `v0.28.39`
  release. Branch commit `09c611e` contains current `origin/main` (`c8053ba`) as
  an ancestor and then merges the exact `v0.28.39` release tag.
- Native target: the Rust-only `aiden-ui` executable rendered by GPUI. The
  Electron renderer remains in the repository solely as the upstream reference;
  `npm run dev:rs` does not load or embed it.
- User-reported before image:
  `/Users/mehmetcanbudak/Desktop/Screenshot 2026-08-15 at 20.16.33 (2).png`.
- The audit covers the returning-user shell and composer, all 12 Settings
  destinations, onboarding provider and finish states, and the root-owned
  Provider, Skill, and MCP editor dialogs.

## Current accepted evidence

### Returning-user shell

- Electron/GPUI main-window pair:
  `/tmp/aiden-parity-postfix/main-pair-final.png` (1512x475), SHA-256
  `ba0420e3112a0e93d4f459e4a1682fa317ae72a5db3b83a547508a674498f94a`.
- The source profile has a workspace while the isolated GPUI profile has no
  folder. The comparison therefore certifies the shared shell, sidebar,
  toolbar, empty canvas, composer measure, controls, colors, and typography;
  the workspace labels are intentionally data-dependent.

### Settings matrix

- Exact Electron route captures at a 1224x768 viewport:
  `/tmp/aiden-electron-providers.jpeg`,
  `/tmp/aiden-electron-model_pad-real.jpeg`,
  `/tmp/aiden-electron-skills-real.jpeg`,
  `/tmp/aiden-electron-mcp-real.jpeg`,
  `/tmp/aiden-electron-web_search-real.jpeg`,
  `/tmp/aiden-electron-scheduled-real.jpeg`,
  `/tmp/aiden-electron-aiden-real.jpeg`,
  `/tmp/aiden-electron-computer_use-real.jpeg`,
  `/tmp/aiden-electron-voice-real.jpeg`,
  `/tmp/aiden-electron-shortcuts-real.jpeg`,
  `/tmp/aiden-electron-appearance.jpeg`, and
  `/tmp/aiden-electron-about-real.jpeg`.
- Current GPUI captures:
  `/tmp/aiden-gpui-providers-current.png`,
  `/tmp/aiden-gpui-modelData-current.png`,
  `/tmp/aiden-gpui-skills-current.png`,
  `/tmp/aiden-gpui-mcp-current.png`,
  `/tmp/aiden-gpui-websearch-current.png`,
  `/tmp/aiden-gpui-scheduledTasks-current.png`,
  `/tmp/aiden-gpui-assistant-current.png`,
  `/tmp/aiden-gpui-computerUse-current.png`,
  `/tmp/aiden-gpui-voice-current.png`,
  `/tmp/aiden-gpui-shortcut-current.png`,
  `/tmp/aiden-gpui-appearance-current.png`, and
  `/tmp/aiden-gpui-about-current.png`.
- Normalized side-by-side pairs are in `/tmp/aiden-parity-final/`. The Agent
  settings contact sheet is
  `/tmp/aiden-parity-final/settings-agent-contact.png` (2448x4608), SHA-256
  `4f4df3db7aa6471f58f3cdab29731c48a9119c1899f93ee457fb7a5c643871fd`.
  The App settings contact sheet is
  `/tmp/aiden-parity-final/settings-app-contact.png` (2448x4608), SHA-256
  `98c19eaa57ce626b94f2356ea9a4dec03b2f7624feea3aa9eb8610c4e449e6dc`.

### Onboarding and root dialogs

- Accepted provider comparison:
  `/tmp/aiden-compare-provider-accepted.png` (4272x1536), SHA-256
  `2239e2f4c87d7c3780e84ccda7e81f19e076b426ca78b6ecee095a2e4b01666a`.
- Accepted finish-card comparison:
  `/tmp/aiden-compare-finish-final2.png` (3440x1200), SHA-256
  `68bc65f01c7acede5e74ba0a2f9f240591ac695290eb8059bce2d755fbc857d9`.
- Current root-owned GPUI dialog captures:
  `/tmp/aiden-gpui-provider-modal-root.png` (3024x1898), SHA-256
  `a2b8a3d0777a1b032cad394079ad7acee77067150941aea7989b3f426a04e333`;
  `/tmp/aiden-gpui-skills-modal-root.png` (3024x1898), SHA-256
  `7a40885e4ee1a19fcdf4932e07ec45d5d6886bdb433cd64ef2a5d13b695dcf41`;
  `/tmp/aiden-gpui-mcp-modal-root.png` (3024x1898), SHA-256
  `4c61284d0df03420432ec2f1c57b16b6434a56b394fa2a7601ac7bef0067dbe0`.

## Route and component result

| Surface | Result | Notes |
| --- | --- | --- |
| Providers | Pass | Built-in inventory, provider rows, Apple/Codex states, controls, select fields, custom-provider dialog, discovery, removal confirmation, and source icons are implemented. |
| Model Pad | Pass | Source header/actions, personal pad, 9x9 empty lattice, ranked model list, benchmark suggestion/source sections, and loading/empty states are implemented. |
| Skills | Pass | Source sections, discovered and managed rows, folder provenance, edit/create dialog, and delete confirmation are implemented. |
| MCP Servers | Pass | Exact two-column popular-card grid, source marks/copy, manual server row, reset action, setup editor, auth/test states, and removal confirmation are implemented. |
| Web Search | Pass | Source field group, switch, credential lifecycle, checking/empty states, and exact draft action copy are implemented. |
| Scheduled Tasks | Pass | Source defaults form, switches, current-task state, create/edit flow, and root delete confirmation are implemented. |
| Aiden | Pass | Global-shortcut card and truthful behavior sections match source hierarchy and tokens. |
| Computer Use | Pass | Beta header, enable/readiness states, permission copy, behavior panel, chips, and clipping bounds match. |
| Voice | Pass | Provider/engine sections, models action, dictation shortcut, permission row, callout, and status states match. |
| Keyboard shortcuts | Pass | Search, global and in-app groups, badges, switches, recorder/reset actions, retained bindings, and source spacing match. |
| Appearance | Pass | Theme previews, code sample, light-theme fields/actions, selects, switch, contrast slider, fonts, colors, radii, and spacing match. |
| About | Pass | Real app mark/version/build, repository/update actions, reset-onboarding copy, and root confirmation match. |
| Main shell/chat | Pass | Titlebar inset, persistent sidebar, nav icons, selection colors, chat measure, assistant message treatment, thinking disclosure, composer, toolbar, and Assistant dock match. |
| Onboarding | Pass | 860x600 shell, 220px rail, six steps, provider density/marks, scroll/footer behavior, and all 22 source feature images match. |

## State boundary

The source and GPUI captures use isolated profiles, so several live values are
not identical: the source profile is signed in and has models, skills, enabled
scheduled tasks, and granted permissions, while the clean GPUI profile can show
`Not configured`, `Listing models`, `Scanning skill folders`, `Checking`, or
disabled defaults. Both sides' corresponding configured, loading, empty,
success, error, and confirmation components were checked against the source
implementation. These are runtime-data differences, not missing layouts.

Chromium and GPUI also produce different subpixel glyph antialiasing. Acceptance
is therefore exact at the component contract level--geometry, hierarchy, assets,
copy, semantic colors, typography metrics, spacing, radii, clipping, states, and
interactions--not a claim that two different renderers emit byte-identical PNGs.

## Findings

- P0: none remaining. Root-owned dialogs correctly occlude the settings content
  and Assistant dock; the Settings Tokio-reactor crash remains fixed.
- P1: none remaining. No audited route, source image, graph/lattice, settings
  section, modal, or primary interaction is omitted.
- P2: none remaining. Shared colors, type metrics, radii, control sizes, spacing,
  iconography, card density, titlebar inset, clipping, and composer geometry were
  reconciled in the comparison loop.

## Verification

- `cargo test --manifest-path rust/Cargo.toml -p aiden-ui` -- 793 passed.
- `cargo test --manifest-path rust/Cargo.toml --workspace` -- passed, including
  all crate and doc tests.
- `cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets
  --all-features -- -D warnings` -- passed.
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` -- passed.
- `npm test` -- passed, including native worktree-remover and Computer Use
  broker gates.
- `npm run type-check` and `npm run lint` -- passed.

final result: passed
