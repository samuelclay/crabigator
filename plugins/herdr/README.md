# Crabigator for Herdr

Run [Crabigator](https://github.com/samuelclay/crabigator) inside a Herdr pane and keep idle / working / blocked in the agent sidebar.

Crabigator wraps Claude Code, Codex, OpenCode, and Grok. Herdr sees the `crabigator` process. Current Crabigator builds report lifecycle state themselves when `HERDR_ENV=1`, and they set the display name to `crabigator · claude` (or `codex` / `opencode` / `grok`).

This plugin is the launcher and PR-board overlay. Agent detection lives in Herdr plus Crabigator's built-in reporter.

## Install

Use a Herdr build that includes Crabigator agent support, and put `crabigator` on your `PATH`.

```sh
herdr plugin link /path/to/crabigator/plugins/herdr
```

From a published repo:

```sh
herdr plugin install owner/crabigator/plugins/herdr
```

`build.sh` records the absolute `crabigator` path so pane commands still work when Herdr's PATH is short.

## Actions

| Action | What it does |
| --- | --- |
| `crabigator.herdr.start` | `herdr agent start` with `--kind crabigator` |
| `crabigator.herdr.start-claude` | Same, passing `claude` through |
| `crabigator.herdr.start-codex` | Same, passing `codex` through |
| `crabigator.herdr.start-opencode` | Same, passing `opencode` through |
| `crabigator.herdr.start-grok` | Same, passing `grok` through |

The `prs` pane entrypoint opens `crabigator prs` as an overlay.

Example keybinding:

```toml
[[keys.command]]
key = "prefix+c"
type = "plugin_action"
command = "crabigator.herdr.start"
description = "start crabigator"
```
