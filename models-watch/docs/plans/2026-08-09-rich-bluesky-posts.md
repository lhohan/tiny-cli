# Plan: Enriched Bluesky posts for newly added models

## Goal

Replace the minimal `New: <id> is now available.` Bluesky post for **added**
models with a data-rich announcement (pricing, context, capabilities,
copyable ID, provider link), sourced from metadata embedded in change deltas
by `models-watch.sh`. Removed and changed posts keep today's format.

## Resolved decisions

1. **Data source**: `models-watch.sh` embeds an additive `"models"` key in
   new delta files — a map of added model ID → full models.dev model object
   (same shape as `state/latest.json` entries). Chosen over render-time
   lookup from `state/latest.json`.
2. **URL line**: provider page —
   `https://models.dev/providers/opencode/` resp. `…/opencode-go/`. Direct
   per-model links (`/models/<lab>/<model>`, verified to exist, e.g.
   `/models/poolside/laguna-s-2.1`) were rejected: the lab ID is not present
   in the watched provider data, and a name-based join against
   `https://models.dev/models.json` fails for 17 of 50 currently watched
   models (opencode exclusives such as `big-pickle` have no canonical page;
   canonical names collide, e.g. "Nano Banana 2").
3. **Vendor line** (e.g. "Poolside"): omitted — not available in structured
   data; it appears only in free-text `description`, and not consistently.
4. **Free detection & signalling**: a model is "free" iff
   `cost.input == 0 and cost.output == 0` (the same filter `models-watch.sh`
   already uses to select Zen models). Free is signalled in **two** places:
   a ` Free` word appended to the display name in the title, and a ` (free)`
   suffix on the pricing line. The `ID:` line is **always** the exact
   provider-prefixed id from the delta (e.g. `opencode/laguna-s-2.1`) — the
   broadcaster never invents a `-free` suffix, so the id stays
   copy-pasteable into opencode config.
5. **Inclusion ladder, not all-or-nothing**: the rich post includes **every
   field we have data for**. If it exceeds 300 code points, the
   least-important still-present optional field is dropped and the text is
   re-measured; this repeats until it fits or no optional field remains.
   There is no separate "malformed → legacy" gate — missing individual
   fields are simply rendered as `–`/omitted, and only the final
   name→id→legacy tail handles residual overflow. (See ladder below.)
6. **Backlog**: the 12 unposted deltas (2026-06-04 → 2026-08-09) are
   recorded as `skipped` in `state/posted.json` with reason
   `pre-rich-format backlog`; the account's first posts use the new format.
   User: "the existing posts do not need to be updated… take effect from the
   new models forward." This intentionally drops the two most recent
   never-posted changes (`2026-07-21`, `2026-08-09`) as well — documented
   loudly in the README.

## Target post format

```text
New on OpenCode Zen: Laguna S 2.1 Free

ID: opencode/laguna-s-2.1
Pricing: $0.00 / $0.00 per 1M tokens (free)
Context: 256K, max output 32K
Capabilities: Tool calling Yes | Structured output No | Reasoning Yes | Attachment support No

https://models.dev/providers/opencode/
```

```text
New on OpenCode Go: Qwen3.8 Max

ID: opencode-go/qwen3.8-max
Pricing: $0.40 / $1.60 per 1M tokens
Context: 1M, max output 65.5K
Capabilities: Tool calling Yes | Structured output No | Reasoning Yes | Attachment support Yes

https://models.dev/providers/opencode-go/
```

Rules:

- Title: `New on <tier>: <name>` where tier is `OpenCode Zen` (`opencode/`
  prefix) or `OpenCode Go` (`opencode-go/` prefix) — a hardcoded two-entry
  map in the broadcaster (delta validation already restricts IDs to these
  two prefixes). Display names match the provider names in `api.json`. A free
  model (both costs `0`) appends ` Free` to `<name>` in the title.
- `ID`: the exact provider-prefixed model ID from the delta
  (`opencode/<slug>` / `opencode-go/<slug>`), copyable into opencode config.
  Never an invented `-free` suffix.
- `Pricing`: `$<input> / $<output> per 1M tokens`, two decimals (`printf
  '%.2f'`); append ` (free)` when both input and output are `0`.
- `Context`: decimal K/M formatting, one decimal trimmed (256000 → `256K`,
  1000000 → `1M`, 65536 → `65.5K`); `max output` from `limit.output`. The
  API exposes no separate max-input figure.
- `Capabilities`: `tool_call`, `structured_output`, `reasoning`,
  `attachment` → `Yes` when true, `No` when false, `–` when the key is
  absent from the model object. Use `has("structured_output")` in jq, not
  an equality check — `has()` returns `false` for a missing key and `true`
  for a present key with any value (`null` included), which matches the
  models.dev convention. `structured_output` is absent on ~70% of watched
  models. Label wording matches the user's requested format:
  `Tool calling`, `Structured output`, `Reasoning`, `Attachment support`.
- Final line: provider page URL, made clickable via an
  `app.bsky.richtext.facet` link facet. Bluesky API posts are not
  auto-linked without facets. The facet uses UTF-8 **byte** offsets:
  `byteStart` = byte length of the text up to and including the blank line
  before the URL; `byteEnd` = total byte length of the rendered text. The
  URL always sits at the end of the text with no trailing whitespace, so
  `byteEnd` equals the total byte length. **Byte lengths are computed with
  `printf '%s' "$text" | wc -c`** (UTF-8 byte count, no trailing newline),
  not jq's `utf8bytelength`, to avoid a jq ≥ 1.7 dependency.
- Limit: 300 Unicode code points. Reference posts measure 271 and 258.
  Overflow is handled by the inclusion ladder (below); every drop action is
  stderr-logged like today.
- Removed (`Removed: <id> is no longer available.`) and changed
  (`Updated: <id>: "old" → "new"`) posts are unchanged.
- RSS feed (`models-feed.sh`) descriptions deliberately stay in the legacy
  format — enriching them is out of scope for this change. The `models` key
  in deltas is silently tolerated (verified: `models-feed.sh` reads only
  `timestamp`/`added`/`changed`/`removed`).

### Inclusion / truncation ladder (rich format)

Include everything we have. While the rendered text exceeds 300 code points
and at least one optional field remains, drop the **least-important
still-present optional field**, rebuild, and re-measure. The dropped-field
priority order (least → most important) is:

1. `Capabilities` line
2. `Context` line
3. `Pricing` line
4. `URL` line and its preceding blank line (a post with no URL also omits
   the `facets` field)

Always retained until the final tail: the title (`New on <tier>: <name>`,
with the ` Free` word when free), the `ID:` line, and the blank-line
separators.

If the text is still over 300 after every optional field is gone, fall back
to the existing tail from the prior format:

5. Shorten the display `<name>` (within the title) using `truncate_value`,
   append `…`, log to stderr, re-measure.
6. Shorten the provider-prefixed `<id>` the same way — a shortened id like
   `opencode-go/qwen3.8…` is still useful for copy-paste. Re-measure.
7. If still over 300, render the legacy format
   `New: <id> is now available.` using the existing ID-truncation logic.

Because free is signalled in the title as well as the pricing line, dropping
the pricing line (step 3) does not erase the free signal. Rebuild and
re-measure uses the **actual rendered text** each step — there is no separate
hand-maintained "overhead" constant.

## Plan (test-first, per repo conventions)

1. **Acceptance tests first** (`tests-rust/tests/acceptance.rs`, DSL in
   `tests-rust/src/lib.rs`):

   - Watcher: a newly written delta contains a `models` object with the
     added model's metadata (add an assertion helper, e.g.
     `expect_delta_model_meta(id)`); the `added` array remains a plain
     string array.
   - Watcher, first-run: when no `latest.json` exists, the delta's `models`
     key contains entries for all watched models.
   - Broadcaster capture, rich path: exact rich text for a free Zen model
     (title carries ` Free`, pricing carries ` (free)`, id exact with no
     `-free`) and a paid Go model; `structured_output` absent → `–`;
     capability label `Attachment support`.
   - Broadcaster capture, **legacy fallback regression (E)**: a delta whose
     added model has **no** `.models` key renders the exact legacy text
     `New: <id> is now available.` and produces the **same ledger hash**
     as today (`4c6990…` from the existing file:// live test) — pins the
     additive, opt-in contract.
   - Broadcaster capture, **inclusion ladder**: a delta whose rich post
     overflows 300 code points asserts the ladder drops fields in priority
     order and always yields ≤ 300. Concretely:
     - capabilities overflow only → `Capabilities` line absent, `Pricing`/
       `Context`/`ID`/URL present;
     - further overflow → `Context` also dropped, then `Pricing`, then URL
       (URL-drop posts omit `facets`);
     - extreme overflow → legacy format with existing ID truncation.
     Use long names/ids to drive overflow; assert substring presence/absence
     and a final `chars().count() <= 300` check.
   - Broadcaster capture, **facets (D)**: capture record includes a
     `facets` array with correct `byteStart`/`byteEnd`/`uri` for the rich
     format, with `byteStart`/`byteEnd` computed via `wc -c` (UTF-8 bytes).
     URL-dropped posts (step 4) omit `facets`.
   - Broadcaster capture, removed/changed: render unchanged.
   - Feed tolerance: `models-feed.sh` processes a delta carrying a `models`
     key without error and produces the same RSS output as today.
   - The existing raw-content helper `with_state_delta(filename, content)`
     suffices for rich delta fixtures; no DSL rewrite needed.

2. **`models-watch.sh`**: when writing a delta, add `"models": {…}`
   containing the full model object for each added ID, taken from the
   freshly built `$current` snapshot (pure `jq`, one extra extraction — the
   `$current` model objects already carry `name`, `cost`, `limit`,
   `tool_call`, `attachment`, `reasoning`, and may omit `structured_output`).
   No other watcher behaviour changes.

3. **`models-broadcast.sh`**:

   - `render_post` receives the **full path** `$df` to read `.models[id]`
     from the delta, but the emitted JSON `"delta"` field keeps the
     **basename** (F) so capture output stays deterministic and testable.
   - `render_post` gains a rich path for `added` when `.models[id]` is
     present; legacy path otherwise. The rich path builds the full text with
     all fields, then applies the inclusion ladder.
   - Provider tier map (hardcoded): `opencode/` → `OpenCode Zen`,
     `opencode-go/` → `OpenCode Go`. Prefix extracted from the model ID.
   - Free title word and ` (free)` pricing suffix when both costs are `0`
     (C). Capability label `Attachment support` (C).
   - Pricing formatting: `printf '%.2f'` for both costs; conditional
     ` (free)` suffix.
   - Context formatting: jq helper for decimal K/M with trailing-zero trim:
     ```jq
     def fmt_ctx:
       if . >= 1000000 then ((. / 1000000 * 10 | floor) / 10) as $v |
         if $v == ($v | floor) then "\($v | floor)M" else "\($v)M" end
       elif . >= 1000 then ((. / 1000 * 10 | floor) / 10) as $v |
         if $v == ($v | floor) then "\($v | floor)K" else "\($v)K" end
       else tostring end;
     ```
   - Capability formatting: each of `tool_call`, `structured_output`,
     `reasoning`, `attachment` → `Yes` when `true`, `No` when `false`, `–`
     when `has("key")` returns `false`.
   - Facet computation (D): `byteStart` = byte length (via
     `printf '%s' | wc -c`) of the text up to and including the blank line
     before the URL; `byteEnd` = total byte length of the rendered text.
     Emit `facets: [{index: {byteStart, byteEnd}, features: [{"$type":
     "app.bsky.richtext.facet#link", uri}]}]` only when the URL line is
     present.
   - `pds_record_body` accepts an optional fourth argument `$facets_json`
     (default `[]`); when non-empty, it merges `facets` into the record
     object; when empty, the `facets` key is **omitted** entirely (B).
   - Capture records gain a `facets` field (present for rich posts with URL,
     absent for legacy and URL-dropped posts).
   - **Live-mode facet forwarding (B)**: the three live `render_post` call
     sites (around lines 561/584/604) must extract `.facets` from
     `post_json` and pass it to `pds_record_body`, so rich posts publish
     with the clickable link. (Coverage: capture-mode facets test proves
     `render_post` produces correct facets; the live loop forwards that same
     value. Add a manual `tmp-preview` eyeball after implementation.)
   - Ledger hashing unchanged (SHA-256 over final post texts only — facets
     derive deterministically from the text, so the hash is unaffected).

4. **Rollout (G — fully developed)**: record the 12 backlog delta
   basenames in `state/posted.json` `skipped` with reason
   `pre-rich-format backlog`:
   - `change-2026-06-04T18:00:01Z.json`
   - `change-2026-06-09T18:00:02Z.json`
   - `change-2026-06-12T18:00:01Z.json`
   - `change-2026-06-13T07:00:01Z.json`
   - `change-2026-06-17T13:00:01Z.json`
   - `change-2026-07-01T09:00:01Z.json`
   - `change-2026-07-02T13:00:03Z.json`
   - `change-2026-07-07T18:00:00Z.json`
   - `change-2026-07-17T07:00:01Z.json`
   - `change-2026-07-18T11:00:02Z.json`
   - `change-2026-07-21T18:00:01Z.json`
   - `change-2026-08-09T07:00:03Z.json`

   This leaves `deltas: {}` and the 11 existing `pre-provider-prefix
   history` skips untouched. The two newest (`2026-07-21`, `2026-08-09`)
   are genuine, never-posted changes that are intentionally dropped per the
   user's "from new models forward" decision; the README must state this
   explicitly so it is not mistaken for a bug.

5. **README.md**: document the new post format with examples, the delta
   `models` key, free detection/signalling (`Free` title word + `(free)`
   pricing, exact id, no invented `-free`), the inclusion ladder, facets and
   byte-offset method, the RSS scope note, and the backlog-skip decision
   (including the intentionally-dropped recent deltas).

## Likely files

- `models-watch/models-watch.sh` — embed `models` key
- `models-watch/models-broadcast.sh` — rich rendering, facets, inclusion
  ladder, full-path reads + basename `delta` field, live facet forwarding
- `models-watch/tests-rust/tests/acceptance.rs` — new scenarios (rich path,
  legacy regression, inclusion ladder, facets)
- `models-watch/tests-rust/src/lib.rs` — one metadata assertion helper
- `models-watch/README.md` — behaviour documentation
- `models-watch/state/posted.json` — rollout skip entries (12 backlog)
- `models-watch/docs/plans/2026-08-09-rich-bluesky-posts.md` — this plan

## Risks

- **300-cap overflow** on future long model names — handled by the inclusion
  ladder: drop `Capabilities` → `Context` → `Pricing` → `URL`, then
  truncate `name` → `id` → legacy. The longest current name+id fits within
  steps 1–2; the ladder guarantees a valid post in all cases.
- **Delta consumers** (`models-feed.sh`, `--report`) read only
  `added`/`removed`/`changed`; the additive `models` key is ignored —
  locked in by the feed tolerance test.
- **Facet byte offsets** must be UTF-8 bytes, not characters — computed with
  `printf '%s' | wc -c` (portable, no jq-version dependency); a miscount
  would render the link unclickable, caught by the facet capture test.
- **Live facet forwarding** is the one piece not covered by an automated
  request-body assertion (the `file://` transport returns fixtures, not the
  outgoing body); mitigated by the capture-mode facets test plus a manual
  `tmp-preview` eyeball. If stronger coverage is wanted later, add a
  request-sidecar env hook — out of scope for this change.
- **Skipped backlog is never announced** anywhere, including the two most
  recent real changes (`2026-07-21`, `2026-08-09`) — accepted by the user
  and documented loudly in the README.
- **`structured_output` absence vs null confusion** — avoided by using
  `has("structured_output")` in jq, which returns `false` for missing keys
  and `true` for `null` or any other value.

## Validation

```bash
bash -n models-watch.sh && bash -n models-broadcast.sh
mise run test   # fallback: cargo test --manifest-path tests-rust/Cargo.toml
./models-broadcast.sh --capture-dir tmp-preview   # eyeball rendered rich posts + facets

# Manual facet byte-offset verification (run after capture mode):
jq '{text, facets}' tmp-preview/1.json
# Confirm byteStart/byteEnd point to the URL substring in the text, in bytes.
```
