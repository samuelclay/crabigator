# Decision ledger

`src/flow/upstream-review.json` is optional until the first review. Use full Git SHAs. It is separate from `FORK.md`'s port baseline.

```json
{
  "version": 1,
  "reviewed_through": "<full upstream SHA with every intervening commit accounted for>",
  "decisions": [
    {
      "id": "surf-barrel",
      "commits": ["<full upstream SHA>"],
      "summary": "Restore the curling wave's barrel",
      "status": "needs-decision",
      "reason": "Explain evidence or the user's actual decision",
      "evidence": ["src/flow/scenes/surf.rs: relevant function"],
      "revisit_when": "Optional trigger for a deferred or skipped decision",
      "rust_commit": null,
      "reference_sha": null,
      "verification": []
    }
  ],
  "scene_references": {}
}
```

Allowed status values: `needs-decision`, `selected`, `ported`, `already-present`, `skipped`, `deferred`. A recommendation alone is not `selected` when a user decision is required. `ported` needs Rust implementation and verification evidence; `scene_references` maps each regenerated scene to the exact TypeScript reference SHA. Keep decisions when advancing the review cursor. The inventory includes unfinished decisions from older ranges.

For a grouped feature, list every constituent commit covered by the decision, including follow-up fixes. An explicit skip should explain why it is irrelevant or declined. A new commit related to a skipped feature should surface for inspection without automatically reopening the settled product choice.
