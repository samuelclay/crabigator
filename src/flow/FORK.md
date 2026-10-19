# flow fork log

`src/flow/` is a Rust port of the scenes from [flow](https://github.com/robdmac/flow), Rob Macrae's ambient-scene plugin for Claude Code, pi and opencode. It is MIT licensed; its license is in `src/flow/LICENSE` and must stay with this code.

flow keeps evolving. This log records which upstream commit the port matches, so each new upstream commit can be read, judged and carried over. Some commits won't apply, such as Claude Code plugin plumbing, sound or `/flow` settings. Many will: new scenes, changes to how a scene looks or moves, the activity model, palettes.

## Where the port stands

| | |
|---|---|
| Upstream | `robdmac/flow` `main` |
| Synced to | `461dea310403ef79d33fa8d6046287f6c454e598` (2026-10-16, PR #32) |
| Plus | the maintained custom Flow fork (`origin`), descended from `crabigator-column`: fade, random scenes, colour palettes, embers instead of a blue pilot light, `isTall` for small boxes. Proposed upstream; until it merges, port from that branch's diff too. |
| Checked by | `make flow-reference FLOW_DIR=../flow` → `src/flow/testdata/*.json`, matched cell for cell by each scene's `frames_match_flow` test |

## How to carry an upstream change over

Use the repository skill [flow-upstream](../../.agents/skills/flow-upstream/SKILL.md) for recurring reviews and selective ports. It inventories the upstream delta, reuses recorded decisions, asks about unclear product choices, and verifies selected Rust ports. Review progress and deferred decisions live in `src/flow/upstream-review.json` when a review is recorded; they do not advance this log's port baseline.

1. Update the flow checkout and list what's new since the commit above:
   ```sh
   git -C ../flow fetch upstream
   git -C ../flow log --oneline 461dea3..upstream/main
   ```
   `origin` is the custom fork; `upstream` must be `robdmac/flow`. Inspect all changed paths, then the relevant patches. Keep the custom TypeScript fork maintained separately; pin the exact reference revision used for a Rust port.
2. For each commit, decide whether it applies (see *What isn't ported* below), then port the change into the matching Rust file (see the map).
3. Re-record the selected scenes' reference frames from a pinned checkout that includes their retained custom behavior, and run the tests. Every scene must match its recorded reference.
   ```sh
   make flow-reference FLOW_DIR=/absolute/pinned/flow SCENES=surf,ski
   cargo test flow::
   ```
   For a complete port, omit `SCENES` to record every scene. Do not replace untouched scenes' fixtures with a revision that has not been ported. A scene whose frames no longer match means the port is behind (or wrong). Never edit the JSON by hand. Record the reference SHA for each batch.
4. Add a row to the log below. Move *Synced to* forward once every commit up to it is ported or marked not applicable.

## Map

| flow (TypeScript) | crabigator (Rust) |
|---|---|
| `hooks/cells.ts` | `src/flow/cells.rs` (no base64 or Raster encoding) |
| `hooks/pixels.ts` | `src/flow/pixels.rs` |
| `hooks/night.ts` | `src/flow/night.rs` |
| `hooks/palette.ts` | `src/flow/palette.rs` |
| `hooks/activity.ts` | `src/flow/activity.rs` (the parts crabigator can feed) |
| `hooks/styles.ts` (`Scene`, `SCENES`), `hooks/scene-def.ts` | `src/flow/scene.rs` |
| `hooks/scene.ts` (`SceneDriver`) | `FlowColumn` in `src/flow.rs` |
| `hooks/sky.ts` | `src/flow/sky.rs` (the `SkyWorld` base class becomes `SkyWorld` + the `SkyScene` trait; the rockets' `LaunchSite` overrides it) |
| `hooks/clouds/layered.ts` | `src/flow/clouds.rs` |
| `hooks/fire.ts`, `hooks/fire-palette.ts`, `Ember` in `hooks/styles.ts` | `src/flow/scenes/fire.rs` |
| `hooks/balloon.ts` | `src/flow/scenes/balloon.rs` |
| `hooks/starfield.ts` | `src/flow/scenes/warp.rs` |
| `hooks/colony.ts` | `src/flow/scenes/avalon.rs` |
| `hooks/engine.ts` | `src/flow/scenes/engine.rs`, `engine/layout.rs`, `engine/draw.rs` |
| `hooks/rocket.ts` | `src/flow/scenes/rocket.rs` (falcon, starship), `rocket/sprites.rs`, `rocket/particles.rs`, `rocket/draw.rs` |
| `hooks/surf.ts` | `src/flow/scenes/surf.rs` |
| `hooks/ski.ts` | `src/flow/scenes/ski.rs`, `ski/band.rs`, `ski/spine.rs` |
| `hooks/bubbles.ts` | `src/flow/scenes/bubbles.rs` |
| `hooks/roadster.ts` | `src/flow/scenes/roadster.rs`, `roadster/draw.rs` |
| `hooks/train.ts` | `src/flow/scenes/train.rs`, `train/{side,above,palette}.rs` |
| `hooks/earthrise.ts` | `src/flow/scenes/earthrise.rs`, `earthrise/world.rs` |
| `hooks/doom.ts` | `src/flow/scenes/doom.rs`, `doom/{draw,sprites,world}.rs` (doom and doom3d) |
| `hooks/crew.ts`, `hooks/agents.ts`, `hooks/waits.ts` | `crew.rs`, `agents.rs`, `hooks.rs`, native platform rosters |
| `hooks/waiting.ts`, `hooks/pixel-scene.ts` | `waiting.rs`, `pixel_scene.rs` |
| `hooks/picker.ts` | `picker.rs` (column UI adapted to Crabigator) |
| `pi/ansi.ts` | `src/flow/ansi.rs` |

## What isn't ported, and why

- **The Claude Code plugin**: `hooks/register.tsx` (except its event handlers, which `src/flow/hooks.rs` follows), `hooks/svg.ts`, `types/`, `.claude-plugin/`. crabigator draws the column itself.
- **pi**: `pi/` (only `ansi.ts` is ported).
- **Sound**: `hooks/sound.ts`, `hooks/sound-files.ts`, `sounds/`, and each scene's `sounds`, `ambience()` and `hear(...)` calls. Also the state that only served sound: avalon's `heard` (and its `leadFrames` check), the twin rocket's `boomed`. Where a scene drew random numbers or moved state while building a sound event, the port keeps that (surf's next-wave draw), so the frames stay the same. If upstream ever times something *drawn* by `leadFrames`, port `leadFrames` with it.
- **What crabigator hears differently**: crabigator isn't a Claude Code plugin, so it hears Claude Code through its hooks instead. The hook appends a line for every hook event to the session's `activity.jsonl`, and `src/flow/hooks.rs` reads it each frame, doing what `register.tsx` does for each event:
  - `PreToolUse` is `tool.call`: the flare by tool (an edit by the lines it writes), and the call in flight until `PostToolUse` or `PostToolUseFailure`. A failed Bash command (not one cut short by Esc) is `failed`.
  - `PermissionRequest` and `Elicitation` report shown permission or elicitation dialogs; the call finishing is the answer. A permission request is matched by call ID or a hash of its arguments (raw input is not retained). An ambiguous request never guesses between concurrent calls of the same tool.
  - `UserPromptSubmit` and `PostToolBatch` stand in for `turn.step` (a model request), and every main-loop event carries the effort. `MessageDisplay` gives the streamed text. It arrives a batch of lines at a time, so `Activity` spreads streamed characters over the following second instead of dropping what a frame can't take. No hook shows thinking, so outside streamed text the assistant's output still stands in (`FlowColumn::hear_output`, about four bytes a character).
  - `SubagentStart` and per-agent work events maintain stable companion IDs in place of `$.agent.list()`. Twenty seconds without work or a running tool means idle; actual questions/permissions mean waiting. Done agents stay six seconds to animate their departure. A subagent Stop never ends the main turn. One silent for ten minutes is dropped, in case its stop was missed. `StopFailure` (an API error) ends the turn with smoke. `PostCompact` is `session.compact`.

  Other assistants use native stats and, where supported, individual-agent rosters. Opencode follows observed child-session events; it does not recover children that finished before attachment. Codex reads direct child rollouts while skipping copied ancestor history. It follows correlated question replies, but child permission dialogs have no verified event and are not inferred from escalation arguments. Grok has no verified individual roster here, so no live companions are invented from its completed-task count. Tool calls are heard when they finish, and the effort comes from Claude Code's banner ("with xhigh effort"). Claude Code's footer ("← 1 agent") can't stand in for running subagents: it counts idle agents too. Context fill (`session.measure`) isn't heard by either path, so the blue tint never shows.
- **Resuming at an altitude**: `SkyWorld.seed` and the rockets' `seed(altitude)` override. A Claude Code reload needed them; the column never reloads. Also `SkyWorld.rowOf`, which nothing calls.
- **Settings and `/flow`**: the plugin's command grammar, shared stores and configuration reload machinery are not copied. Crabigator's Ctrl+] cycle includes Settings with a scene checklist, on/off, fade, activity level, day/night and color controls. Save applies to the current session and future sessions; Esc discards the draft. It stays inside the existing column. Space or Enter toggles scene checkboxes; unchecked scenes are excluded from startup selection and cycling. Left/right edits controls below the checklist; Enter on Save saves, and End jumps to Save. Ctrl+] discards the draft and advances to Off. Remote dashboard input continues to target the assistant, while local keys and paste belong to the open picker.
  - Fade defaults off; idle remains a glow. Each new session claims an enabled scene rather than using a shared shuffle bag. Named choices last only for that session; only Off persists.
  - Omitted `level` follows activity; a manual 0–10 value suppresses activity tints and waiting, while companions still show.
  - Omitted `night` follows the local clock; true/false forces night/day.
  - `color` defaults to `session`; `natural` or named hues override it on scenes with palette metadata. Train, Earthrise, Doom and Doom3d also support recoloring, by the user's choice; `natural` retains upstream colors.
  - Settings live in `[flow]` in `~/.crabigator/config.toml`; `disabled_scenes` stores unchecked scene names. A disabled current selection falls back to an enabled scene claim. Old saved `scene` fields are ignored and removed on the next config save. Leave only one scene checked to use it in every session.
- **Desktop images and hover cards**: no PNG/SVG pipeline, interactive agent marks or task popovers in the terminal column. Earthrise uses its cell renderer. Its extra MIT notice is in `THIRD-PARTY-NOTICES`.
- **Tools**: upstream preview/check/scene scaffolding and sound builds stay in Flow. Crabigator has `scripts/flow-reference.ts`, `src/flow/preview.rs` and `scripts/flow-preview.py` for its own replay, visual review and timings.

## crabigator's own additions

Not in flow, so keep them when porting upstream changes:
- **ctrl+]** cycles enabled scenes → Settings → Off → the first enabled scene. The user restored cycling after trying the picker toggle. Its off label remains available to turn it back on. The picker supports the existing 15–40-column, 4–12-row box; it does not enlarge Doom3d's column. With every scene unchecked, Settings and Off remain reachable.
- **New scene palettes**: Train and Earthrise gently recolor blue regions; Doom and Doom3d recolor brown architecture. Land, signal lamps, combat colors and neutral terrain stay recognizable. These Rust-only palette rules preserve upstream colors under `natural` and do not recolor waiting frames.
- **Train in small boxes**: the custom TypeScript fork's `2e1785c` reduces empty track ahead and companion sway, and accepts a visible cab instead of requiring half a two-car unit. The locomotive and one companion remain readable in short columns; full band/spine layouts retain upstream's geometry.
- Enabled scenes are claimed per session (`claim.rs`), and the colour comes from the session mark.
- **Figures wear the session's hue** (`Dials.accent`, `SceneDef.figure`): the balloon's stripes, the hero skier's hat and jacket, the hero surfer's board, the rockets' white and stainless bodies (a wash), Dragon's trunk and parachute gores. Those scenes keep their frame's own colours (no palette turn), so their skies stay blue. With no accent, every scene draws exactly flow's frames. When porting an upstream change to balloon, ski, surf or rocket, keep the `accent` lines.

## How the port stays exact

flow's scenes are deterministic from a seed, so the port draws the same cells. The rules, kept in `src/flow/js.rs`:
- `Math.round` rounds halves up, so use `js::round`, never `f64::round`.
- `x | 0` → `i32_of`, `>>> 0` → `u32_of`, `Math.imul` → `imul`.
- `Float32Array` values are stored as `f32`; `Uint8Array` and `Uint16Array` values wrap.
- The random number generator is drawn in the same order. Object literals that call it are evaluated in source order.
- `Math.hypot` → `js::hypot`, which copies V8's algorithm (libm's `hypot` rounds differently about a third of the time).
- A write past the end of a typed array does nothing (`Cells::set`).

## Log

| Upstream commit | What | Status |
|---|---|---|
| `7d6d6cf`…`e70a702` | Everything up to and including PR #5: the ten scenes, the activity model, sky and clouds | Ported: all ten scenes match the reference frames (six runs each: the column's sizes, the 5-row band, a 22×60 spine; day and night; smoke, blue, subagents) |
| `crabigator-column` (flow branch) | Fade; random scenes and colours; per-scene palettes (`SceneDef.hue`); embers at level 1 and a blue low fire under the context-full tint; `isTall` for boxes up to 12 rows; `CRABIGATOR_FLOW` hides the plugin's band | Ported, except random scenes and colours (crabigator claims scenes per session instead) |
| `1a10bb9` (flow branch) | Fade: a long turn still climbs (never below plain auto while a turn runs) | Ported |
| `e70a702..b3676b8` | All fourteen scenes, waiting behavior, individual companions, scene/settings picker and rendering fixes | Ported or explicitly excluded above; all 124 commits have a disposition in the ledger, including 21 inspected merges |
| `2e1785c` (custom Flow fork) | Keep Train's locomotive and companions visible in short columns | Ported and matched against this revision's frames |

The reference is custom merge `b8a606fb1f1f8d22b315b25647c8824591bec8f7`, which includes upstream `b3676b875e9bd37aea132941bf99197453d7f525` and the retained custom patches. Train uses `2e1785c3f63399d5b1fcde132d1ce22ffc3afa82`. `upstream-review.json` records complete SHAs, dispositions and verification. Native adapters are deliberate integrations with Crabigator's event sources.

Verification includes exact reference frames for all fourteen scenes, long Avalon and rocket missions, real terminal picker navigation/cancellation and input ownership at 15×4, native companion lifecycle tests, a live Codex child-roster probe, and Rust visual previews across 15×4, 24×8 and 40×12. The TypeScript fork passes its 201 plugin tests, strict validation and scene check.

Final Rust validation: 707 tests passed, zero failed, nine manual tests ignored; release build and Clippy passed. Clippy reports existing warnings outside this port. Formatting passes for all changed Rust files; the full repository formatting check still reports existing formatting in `pr.rs`, `pr_rank.rs` and `prs_board.rs`. The port and native adapters are committed in `daf6f7f` and `04a5770`.


### 2026-10-19 update

| Upstream commit | What | Status |
|---|---|---|
| `b3676b8..461dea3` | Roadster, repeatable redraws, shared rendering helpers, and host cleanup | All 18 commits inspected and recorded in the ledger; applicable scene changes ported |
| `59c219a`, `ac6b23b`, `28bbbc0`, `14783bf`, `6250b49` | Roadster's town/freeway driving, rolls, companions, clouds and white body | Ported in the existing column with the shared small-box layout; no extra pane height |
| `40a8454` | Engine firebox and rocket plume flicker use frame hashes | Ported; drawing repeatedly or skipping draws leaves subsequent animation unchanged |
| `08ce5a1` | Shared hashes, easing, fire ramps and layout rule | Ported where Rust had a gap; Rust already clamps scene levels |
| Crabigator selection change | Temporary named choices, sticky Off, no Random entry or marker | New sessions claim enabled scenes; the checklist controls persistent scene preferences |

The new reference is the sibling fork's merge `03bb2cdde7a88b4020738a8ec75920b68eabde66`, pinned in a detached checkout. It includes upstream `461dea310403ef79d33fa8d6046287f6c454e598` and the retained custom changes. Fire, Engine, Falcon, Starship and Roadster fixtures use this revision. Other fixtures retain their previous sources; Train still uses `2e1785c3f63399d5b1fcde132d1ce22ffc3afa82`.

Roadster retains the sound lead and frame duration in its movement and jumps: those values affect visible motion. Sound-only events and bookkeeping, hover marks, plugin settings/packaging, and audio assets stay excluded. Roadster keeps its white body and original palette, including colored companion cars, visible lights while waiting, and weather tints.

Verification: all fifteen scene reference tests pass with zero cell differences, including eight Roadster trips through 3,400 frames. Roadster behavior tests cover parked companions, traffic clearance, waiting and departure at 15×4, 40×12, 120×5 and 22×60. The redraw regression covers all scenes. Actual Rust previews cover 15×4, 24×8 and 40×12 across idle, busy, day/night, waiting, companions, smoke and blue; Roadster's largest sampled p95 is 0.032 ms. The real tmux picker check uses the 15×4 column and isolated settings, covering startup migration, cycling, save/cancel, paste ownership and Off.

Final checks for this update: debug and release builds pass; Clippy passes with warnings only in unchanged files. The full suite reports 747 passed, 11 ignored and one pre-existing PR-board layout failure (`title_takes_the_slack_link_room_and_the_diff_hugs_the_activity`), reproduced independently on starting commit `8cb276a`. Changed Rust formatting and `git diff --check` pass. The required simplifier review found no further changes. A fresh terminal session confirmed Off persists; another showed Roadster and its checkbox in the 15×4 column. Violet previews confirmed the existing Fire, Engine and rocket color rules remain intact.
