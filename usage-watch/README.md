# usage-watch

A small CLI that prints **remaining usage** for AI subscriptions. Credentials come
purely from environment variables; every provider whose variable is set is queried
and reported.

Supported subscriptions:

| Provider ID   | Subscription            | Env vars                                                        |
| ------------- | ----------------------- | --------------------------------------------------------------- |
| `opencode-go` | OpenCode Go             | `OPENCODE_GO_API_KEY` or `OPENCODE_API_KEY`                     |
| `chatgpt`     | ChatGPT Go / Plus / Pro | `CHATGPT_ACCESS_TOKEN` (+ optional `CHATGPT_ACCOUNT_ID`)        |
| `mistral`     | Mistral La Plateforme   | `MISTRAL_API_KEY` (needs billing scope)                         |
| `vibe`        | Mistral Vibe (Le Chat)  | `VIBE_API_KEY` or `MISTRAL_API_KEY` (+ optional `MISTRAL_ADMIN_COOKIE`) |

## Usage

```bash
usage-watch                 # all providers whose env var is set
usage-watch -p opencode-go  # just one (repeatable)
usage-watch --json          # machine-readable output
usage-watch -t 30           # per-request timeout in seconds (default 15)
```

Exit codes: `0` everything OK, `1` at least one provider request failed, `2` bad
arguments or no provider configured.

Example output:

```text
OpenCode Go [opencode-go] — plan: subscribed 2026-05-22T14:30:00Z
  5h window      $9.66 of $12.00 left (19.5% used) — resets in 2h00m
  weekly         $21.09 of $30.00 left (29.7% used) — resets in 4d00h
  monthly        $45.00 of $60.00 left (25.0% used) — resets in 16d07h

ChatGPT [chatgpt] — plan: plus
  primary (5h00m)  26% left (74.0% used) — resets in 1h43m
  secondary (7d00h)  73% left (27.0% used) — resets in 3d02h
```

## Getting the credentials

- **OpenCode Go**: create an API key in the OpenCode console and export it as
  `OPENCODE_API_KEY` (or `OPENCODE_GO_API_KEY`).
- **ChatGPT**: the tool calls the same endpoint the Codex CLI uses
  (`https://chatgpt.com/backend-api/wham/usage`, unofficial). Use the access token
  from a ChatGPT sign-in, e.g. `jq -r '.tokens.access_token' ~/.codex/auth.json`,
  exported as `CHATGPT_ACCESS_TOKEN`. If your account has multiple workspaces, also
  export `CHATGPT_ACCOUNT_ID`.
- **Mistral (La Plateforme)**: export an org API key as `MISTRAL_API_KEY`. The key
  must have billing scope for `GET /v1/billing/subscription` and
  `GET /v1/billing/usage`.
- **Vibe**: export the Vibe CLI key from <https://console.mistral.ai/codestral/cli>
  as `VIBE_API_KEY` (plan info). For the monthly Vibe plan percentage and credit
  balance, paste a full `Cookie:` header from a request to `admin.mistral.ai` into
  `MISTRAL_ADMIN_COOKIE` (must include an `ory_session_*` cookie; `csrftoken` enables
  the usage endpoints).

## Build

```bash
cargo build --release   # binary at target/release/usage-watch
cargo test
```

## Notes

- ChatGPT's `wham/usage` and Mistral's console/trpc endpoints are unofficial and can
  change without notice; the parsers are tolerant and degrade to an error line.
- Mistral amounts are EUR unless the API says otherwise; OpenCode Go amounts are USD.
