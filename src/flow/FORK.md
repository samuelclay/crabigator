# flow fork log

`src/flow/` is a Rust port of the scenes from [flow](https://github.com/robdmac/flow), Rob Macrae's ambient-scene plugin for Claude Code and pi. It is MIT licensed; its license is in `src/flow/LICENSE` and must stay with this code.

flow keeps evolving. This log records which upstream commit the port matches, so each new upstream commit can be read, judged and carried over. Some commits won't apply, such as Claude Code plugin plumbing, sound or `/flow` settings. Many will: new scenes, changes to how a scene looks or moves, the activity model, palettes.

## Where the port stands

| | |
|---|---|
| Upstream | `robdmac/flow` `main` |
| Synced to | `e70a702` (2026-10-06, "Merge pull request #5 from robdmac/improvements") |
| Plus | the `crabigator-column` branch of flow: fade, random scenes, colour palettes, embers instead of a blue pilot light, `isTall` for small boxes. Proposed upstream; until it merges, port from that branch's diff too. |
| Checked by | `make flow-reference FLOW_DIR=../flow` → `src/flow/testdata/*.json`, matched cell for cell by each scene's `frames_match_flow` test |

## How to carry an upstream change over

1. Update the flow checkout and list what's new since the commit above:
   ```sh
   git -C ../flow fetch && git -C ../flow log --oneline e70a702..origin/main -- hooks/
   ```
2. For each commit, decide whether it applies (see *What isn't ported* below), then port the change into the matching Rust file (see the map).
3. Re-record the reference frames from the updated checkout, and run the tests. Every scene must match.
   ```sh
   make flow-reference FLOW_DIR=../flow
   cargo test flow::
   ```
   A scene whose frames no longer match means the port is behind (or wrong). Never edit the JSON by hand.
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
| `pi/ansi.ts` | `src/flow/ansi.rs` |

## What isn't ported, and why

- **The Claude Code plugin**: `hooks/register.tsx` (except its event handlers, which `src/flow/hooks.rs` follows), `hooks/svg.ts`, `types/`, `.claude-plugin/`. crabigator draws the column itself.
- **pi**: `pi/` (only `ansi.ts` is ported).
- **Sound**: `hooks/sound.ts`, `hooks/sound-files.ts`, `sounds/`, and each scene's `sounds`, `ambience()` and `hear(...)` calls. Also the state that only served sound: avalon's `heard` (and its `leadFrames` check), the twin rocket's `boomed`. Where a scene drew random numbers or moved state while building a sound event, the port keeps that (surf's next-wave draw), so the frames stay the same. If upstream ever times something *drawn* by `leadFrames`, port `leadFrames` with it.
- **What crabigator hears differently**: crabigator isn't a Claude Code plugin, so it hears Claude Code through its hooks instead. The hook appends a line for every hook event to the session's `activity.jsonl`, and `src/flow/hooks.rs` reads it each frame, doing what `register.tsx` does for each event:
  - `PreToolUse` is `tool.call`: the flare by tool (an edit by the lines it writes), and the call in flight until `PostToolUse` or `PostToolUseFailure`. A failed Bash command (not one cut short by Esc) is `failed`.
  - `PermissionRequest` and `Elicitation` are a `tool.check` that asked; the call finishing is the answer. A permission request names no call, so it marks the main loop's newest call of that tool.
  - `UserPromptSubmit` and `PostToolBatch` stand in for `turn.step` (a model request), and every main-loop event carries the effort. `MessageDisplay` gives the streamed text. It arrives a batch of lines at a time, so `Activity` spreads streamed characters over the following second instead of dropping what a frame can't take. No hook shows thinking, so outside streamed text the assistant's output still stands in (`FlowColumn::hear_output`, about four bytes a character).
  - `SubagentStart` and `SubagentStop` count running subagents in place of `$.agent.list()`. One silent for ten minutes is dropped, in case its stop was missed. `StopFailure` (an API error) ends the turn with smoke. `PostCompact` is `session.compact`.

  Other assistants have no such log: their stats stand in. Tool calls are heard when they finish, and the effort comes from Claude Code's banner ("with xhigh effort"). Claude Code's footer ("← 1 agent") can't stand in for running subagents: it counts idle agents too. Context fill (`session.measure`) isn't heard by either path, so the blue tint never shows.
- **Resuming at an altitude**: `SkyWorld.seed` and the rockets' `seed(altitude)` override. A Claude Code reload needed them; the column never reloads. Also `SkyWorld.rowOf`, which nothing calls.
- **Settings and `/flow`**: `hooks/settings.ts`, `hooks/pick.ts`. In crabigator the choices are fixed:
  - fade is off unless `[flow] fade = true`, as in flow, where it is off by default;
  - idle is a glow, never dark;
  - the scene is claimed per session (`src/flow/claim.rs`) instead of drawn from a shuffle bag;
  - the colour comes from the session mark, not a setting;
  - day and night follow the clock.

  The only settings are `[flow] enabled` and `[flow] fade` in `~/.crabigator/config.toml`.
- **Tools**: `scripts/` (preview, check, sync-manifest, new-scene, make-sounds). `scripts/flow-reference.ts` here plays the part `check` plays there.
- **`hooks/pixel-scene.ts`**: no scene uses it yet. Port it with the first scene that does.

## crabigator's own additions

Not in flow, so keep them when porting upstream changes:
- **ctrl+]** rotates the column's scene (`FlowColumn::next_scene`), and the separator above the column names the scene and the key.
- Scenes are claimed per session (`claim.rs`), and the colour comes from the session mark.
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
