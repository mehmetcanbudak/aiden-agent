# GPUI parity design QA

## Comparison target

- Source visual truth: current merged Main (`c8053ba`, tag `v0.28.36`) in
  `renderer/components/onboarding-flow.tsx`, the shared renderer design tokens,
  and the 22 shipped images in `renderer/assets/onboarding/features/`.
- Native implementation: the Rust-only `aiden-ui` executable rendered by GPUI.
  The Electron/React implementation remains in the repository as the upstream
  reference, but it is not loaded by `npm run dev:rs`.
- Reported pre-fix screenshots:
  `/var/folders/xm/66q9p6h91vs2w2l6fdl2j4z00000gn/T/TemporaryItems/NSIRD_screencaptureui_ewEP2H/Screenshot 2026-08-15 at 17.58.55.png`
  and
  `/var/folders/xm/66q9p6h91vs2w2l6fdl2j4z00000gn/T/TemporaryItems/NSIRD_screencaptureui_8AuJrU/Screenshot 2026-08-15 at 18.00.59.png`.
- Provider comparison: Electron and GPUI were captured in the same light-theme
  provider state at 2136x1536, then placed side by side without scaling.
- Finish comparison: the source window capture used a different outer window
  size, so the identical 1720x1200 onboarding-card region was cropped from each
  capture and placed side by side without scaling. This is a component-level,
  not full-window, comparison.

## Accepted evidence

- Electron provider: `/tmp/aiden-electron-provider-front.png` (2136x1536).
- GPUI provider: `/tmp/aiden-gpui-provider-accepted.png` (2136x1536).
- Combined provider comparison:
  `/tmp/aiden-compare-provider-accepted.png` (4272x1536), SHA-256
  `2239e2f4c87d7c3780e84ccda7e81f19e076b426ca78b6ecee095a2e4b01666a`.
- Electron finish source: `/tmp/aiden-electron-tour.png` (2224x1624).
- GPUI finish: `/tmp/aiden-gpui-finish-final.png` (2136x1536).
- Combined finish-card comparison:
  `/tmp/aiden-compare-finish-final2.png` (3440x1200), SHA-256
  `68bc65f01c7acede5e74ba0a2f9f240591ac695290eb8059bce2d755fbc857d9`.
- Live Settings regression capture:
  `/tmp/aiden-gpui-settings-regression.png` (2224x1624), SHA-256
  `04dfb0736d4d068692f974ea207fe0836bb1ca45111fdb80c1f411fb49fb9a79`.

## Findings

- P0: none remaining in the audited flow. Opening Settings originally crashed
  the GPUI app because Foundation Models status work was polled on GPUI's
  generic background executor without a Tokio reactor. Settings boot and the
  same latent first-turn title path now dispatch through the Tokio bridge.
  Repeating the exact launch/open/wait sequence kept the app alive for ten
  seconds and showed Apple Foundation Models as Ready.
- P1: none remaining. The 860x600 card, 220px rail, six steps, provider order,
  provider density, current copy, clipped child surfaces, scrolling body,
  fixed footer, grouped bento layout, and all 22 illustrations match the
  current source contract in the accepted comparisons.
- P2: none remaining. Heading/icon alignment, source Lucide icons, 13/17 and
  12/16 text metrics, 16px outer and 12px inner radii, source input/well tokens,
  and pill actions were corrected during the comparison loop.
- Expected renderer variance: Chromium and GPUI rasterize text differently at
  the subpixel level. The accepted result matches visible geometry, hierarchy,
  assets, states, and interaction contracts; it does not claim byte-identical
  screenshots across two rendering engines.

## Required fidelity surfaces

- Fonts and typography: accepted at the target scale; no visible hierarchy or
  wrapping mismatch remains.
- Spacing and layout rhythm: accepted for provider full view and finish-card
  region, including the compact scroll/footer behavior that triggered the
  final recapture.
- Colors and visual tokens: current source semantic light tokens are used; the
  development-only appearance seam makes the comparison deterministic.
- Image quality and asset fidelity: all 22 current source PNGs are embedded
  directly. Visible control icons come from the installed source Lucide set,
  not approximations or hand-drawn substitutes.
- Copy and content: current Main step labels, headings, descriptions, provider
  catalog/order, and completion-tour copy are present.
- Interaction: provider selection/setup, More-provider disclosure, scrolling,
  back/skip/continue/finish actions, API-key/local discovery paths, and
  ChatGPT OAuth entry are native GPUI controls.

## Comparison history

1. The reported build showed the old sparse progress treatment, missing
   illustrations/provider marks, and chat-control regressions.
2. The first source-versus-GPUI captures exposed remaining density, icon,
   radius, clipping, and type-scale differences.
3. GPUI was corrected to use the source geometry, current Lucide assets,
   compact provider cards, exact scroll/footer separation, source copy, and
   the complete 22-image finish gallery.
4. The provider full-view and finish card-level combined comparisons were
   recaptured after the compact-card/scroll fix. Review found no remaining
   P0/P1/P2 mismatch in those states.
5. Functional reproduction found the unrelated Settings reactor crash; both
   affected executor boundaries were fixed and regression-tested before final
   acceptance.

## Verification

- Full `npm test`, including its pretest suites and native worktree-remover and
  Computer Use broker gates: passed.
- `npm run type-check`: passed.
- `npm run lint`: passed.
- Full Rust workspace tests, strict all-target/all-feature Clippy, rustfmt, and
  diff validation are recorded in the parity plan and project history.

final result: passed
