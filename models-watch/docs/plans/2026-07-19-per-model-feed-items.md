# Plan: Per-model RSS feed items in `models-feed.sh`

## Goal

Change `models-watch/models-feed.sh` so the RSS feed contains **one `<item>` per affected model**, per change event (added / changed / removed), instead of the current one item per delta file that bundles all models together.

## Assumptions

1. **Granularity:** each (delta × action × model) row becomes its own `<item>`.
2. **Feed window:** the last 100 **`<item>` elements total**, newest-first.
3. **Ordering within a delta:** Added → Changed → Removed, each group in delta-file order. Across deltas: newest first.
4. **GUID:** `models-watch-<ISO timestamp>-<action>-<model-id>` (with model-id XML-escaped).
5. **Title:** `Added: <model-id>` / `Changed: <model-id> — "<old>" → "<new>"` / `Removed: <model-id>`.
6. **Description:** one focused line per item, CDATA-wrapped.
7. **pubDate:** shared delta timestamp.
8. **Empty deltas (all arrays empty):** produce zero items.
9. **Exit codes, flags, env, atomic-write, RSS 2.0 envelope, `atom:link`** unchanged.
10. `state/change-*.json` delta format is unchanged; only the feed rendering changes.

## Plan

1. Rewrite the item-generation loop in `models-feed.sh`: iterate `added[]`, `changed[]`, `removed[]` per delta and emit one `<item>` per row, with unique guid, per-model title/description, and a 100-item cap.
2. Update acceptance tests in `acceptance_rss.rs` for per-model granularity and the 100-item cap.
3. Update `README.md` feed-format section.

## Likely files

- `models-watch/models-feed.sh`
- `models-watch/tests-rust/tests/acceptance_rss.rs`
- `models-watch/README.md`

## Validation

- `bash -n models-watch/models-feed.sh`
- `mise run test` (fallback: `cargo test --manifest-path tests-rust/Cargo.toml`)
