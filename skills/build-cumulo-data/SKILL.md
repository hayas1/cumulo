---
name: build-cumulo-data
description: Build or update cumulo's data (the import JSON) from a browser's bookmarks/history export. Use when you want the consoles you visit for operations (BigQuery, Error Reporting, Cloud Logging, GKE, ...) organized by project or purpose.
---

# build-cumulo-data

Turn a browser's bookmarks/history into cumulo's import JSON — new, or updated in place.

Read the domain (what counts as a Resource/Category and why it is classified that way) from `docs/domain.md` and `docs/README.md`; it is not repeated here. For the output format, treat `cumulo-model/src/demo/cloud.json` — which the implementation round-trips — as the single source of truth and match it.

## Where the browser data lives

Chrome's default profile keeps two files (other profiles are siblings of `Default/`):

- `Bookmarks` — JSON, read directly.
- `History` — SQLite (`urls`, `visits` tables). It is locked while Chrome is running, so copy the file first or close Chrome.

Typical default locations:

- Linux: `~/.config/google-chrome/Default/{Bookmarks,History}`
- Windows (from WSL): `/mnt/c/Users/<you>/AppData/Local/Google/Chrome/User Data/Default/{Bookmarks,History}`
- macOS: `~/Library/Application Support/Google/Chrome/Default/{Bookmarks,History}`

## Inputs

- **Required**: the browser's bookmarks and/or history (either or both).
- **Required except on the first run**: the current cumulo data JSON (the existing import file).

## Modes

- **First run (only materials, or the target is the demo data)**: build from scratch. Do not carry the demo over as a base.
- **Otherwise**: **update and merge** onto the current cumulo data. Keep the hand-tuned labels, classifications, and taxonomy structure; only add what the new export introduces. Identify existing Resources by URL and do not drop them (keep them even if they no longer appear in the materials).

## Steps

1. From the material URLs, pick out links to the consoles seen during operations.
2. Decide which values (service / project / env, ...) each URL belongs to **by inference**. Do not build or keep a URL-to-category table — that is a settled decision not to implement, because it does not scale. Handle unseen URL shapes by inference too.
3. Fill gaps from actual usage in history even when a link was never bookmarked. You may also fill zero-signal gaps by inference ("looks like GKE is in use"), but keep this conservative and make anything added by guess easy for a human to remove.

## Output and handoff

- Write the import JSON with the same schema as `cloud.json` to a file.
- Summarize what was produced or changed — especially anything added by inference — and have a human review it. The human makes the final call.
