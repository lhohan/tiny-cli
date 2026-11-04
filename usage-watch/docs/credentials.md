# Credentials

Every provider reads its credentials from environment variables. This page is
the exact procedure for each one: where the value comes from, how to set it, and
what to do when it stops working.

| Provider      | Required                                    | Optional               | Expires                     |
| ------------- | ------------------------------------------- | ---------------------- | --------------------------- |
| `opencode-go` | `OPENCODE_GO_API_KEY` or `OPENCODE_API_KEY` | —                      | never (revoke only)         |
| `chatgpt`     | `CHATGPT_ACCESS_TOKEN`                      | `CHATGPT_ACCOUNT_ID`   | hours                       |
| `mistral`     | `MISTRAL_API_KEY`                           | `MISTRAL_ADMIN_COOKIE` | key: never, cookie: 90 days |
| `vibe`        | `VIBE_API_KEY` or `MISTRAL_API_KEY`         | `MISTRAL_ADMIN_COOKIE` | key: never, cookie: 90 days |

For one shell session:

```fish
set -gx OPENCODE_API_KEY 'sk-...'
```

```bash
export OPENCODE_API_KEY='sk-...'
```

To make it permanent, see [Storing credentials](#storing-credentials).

## OpenCode Go

The environment variable is all you need. No cookie, no second value.

1. Sign in at <https://opencode.ai/console> and subscribe to Go or Go Plus.
2. Open <https://opencode.ai/console/keys>, create a key, copy it.
3. Set it:

   ```fish
   set -gx OPENCODE_API_KEY 'sk-...'
   ```

4. Check:

   ```bash
   usage-watch -p opencode-go
   ```

`OPENCODE_GO_API_KEY` is also accepted and wins when both are set. This is the
same key OpenCode itself uses (`/connect` → OpenCode Go), so if OpenCode works
on this machine, the key already exists.

The key does not expire. It stops working when the subscription lapses or you
revoke it, and the tool then prints `HTTP 401` with `api_key_expired` in the
body. Create a new key and repeat step 3.

## ChatGPT

The tool calls the endpoint the Codex CLI uses, so the credential comes from a
Codex sign-in.

1. Sign in:

   ```bash
   codex login
   ```

2. Read the token out of `~/.codex/auth.json`:

   ```bash
   jq -r '.tokens.access_token' ~/.codex/auth.json
   ```

3. Set it, plus the account id when the account has several workspaces:

   ```fish
   set -gx CHATGPT_ACCESS_TOKEN (jq -r '.tokens.access_token' ~/.codex/auth.json)
   set -gx CHATGPT_ACCOUNT_ID (jq -r '.tokens.account_id // empty' ~/.codex/auth.json)
   ```

4. Check:

   ```bash
   usage-watch -p chatgpt
   ```

The access token is short-lived and Codex refreshes it in place, so a value
exported once goes stale. Read it at call time instead:

```fish
function usage-watch-chatgpt
    set -lx CHATGPT_ACCESS_TOKEN (jq -r '.tokens.access_token' ~/.codex/auth.json)
    set -lx CHATGPT_ACCOUNT_ID (jq -r '.tokens.account_id // empty' ~/.codex/auth.json)
    usage-watch -p chatgpt $argv
end
```

A stale token shows up as
`HTTP 401: {"detail":"Could not parse your authentication token..."}`.

## Mistral La Plateforme

Two values with two different jobs:

| Variable               | Unlocks                                                 |
| ---------------------- | ------------------------------------------------------- |
| `MISTRAL_API_KEY`      | plan name, from `GET /v1/billing/subscription`          |
| `MISTRAL_ADMIN_COOKIE` | monthly tokens and credit balance, from the web session |

The Admin API (`/v1/admin/usage`) is Enterprise-only, so a normal API key cannot
read usage. The browser session can.

### API key

1. Open <https://console.mistral.ai/api-keys> and create a key.
2. Set it:

   ```fish
   set -gx MISTRAL_API_KEY '...'
   ```

3. Check:

   ```bash
   usage-watch -p mistral
   ```

   You get the plan line and a note to set the cookie.

### Cookie

1. Sign in at <https://admin.mistral.ai>.
2. Open DevTools → **Network**.
3. Reload <https://admin.mistral.ai/organization/usage>.
4. Click any request to `admin.mistral.ai`.
5. **Request Headers** → `Cookie` → copy the whole value.
6. Paste it into the helper:

   ```bash
   ./scripts/mistral-cookie.sh
   ```

   It checks the session, runs both Mistral providers, and prints the keychain
   command. Add `--save` to store the cookie in the keychain.

Or set it by hand:

```fish
set -gx MISTRAL_ADMIN_COOKIE 'ory_session_...=...; csrftoken=...'
```

Only two cookies matter: `ory_session_*` (the login) and `csrftoken` (CSRF for
the usage endpoints). `__cf_bm`, `intercom-*`, `browser-session-id` and
`csrf_token_*` are noise; leaving them in is harmless.

## Vibe

Same cookie as above, different key.

1. Open <https://console.mistral.ai/codestral/cli> and copy the Vibe key.
2. Set it:

   ```fish
   set -gx VIBE_API_KEY '...'
   ```

   `MISTRAL_API_KEY` works too, so a machine with only that key still reports
   the Vibe plan.

3. Set the cookie, the same value as for La Plateforme:

   ```fish
   set -gx MISTRAL_ADMIN_COOKIE '...'
   ```

4. Check:

   ```bash
   usage-watch -p vibe
   ```

Without the cookie you get the plan but not the monthly percentage or credits.

## When the cookie expires

The session lasts 90 days from sign-in. It also dies earlier if you sign out or
Mistral rotates it.

Symptoms:

- `usage failed: too many redirects` — the request bounced to `auth.mistral.ai`.
- `HTTP 401` from `billing/v2/usage`.

Fix, exactly:

1. Sign in at <https://admin.mistral.ai>.
2. DevTools → **Network** → reload <https://admin.mistral.ai/organization/usage>.
3. Copy the `Cookie` request header from any `admin.mistral.ai` request.
4. Run:

   ```bash
   ./scripts/mistral-cookie.sh --save
   ```

5. Check:

   ```bash
   usage-watch -p mistral -p vibe
   ```

`--save` overwrites the keychain item, so nothing in the dotfiles changes.

## Storing credentials

Values live in the macOS keychain and fish reads them at shell start. Nothing
secret goes into the repo.

1. Add the item. `security` prompts twice, which keeps the value out of shell
   history and out of `ps`:

   ```bash
   security add-generic-password -U -s "mistral-admin-cookie" -a "$USER" -w
   ```

2. Add a block to `packages/home-manager/.config/home-manager/fish.nix`, inside
   `interactiveShellInit`:

   ```fish
   if not set -q MISTRAL_ADMIN_COOKIE
     set -gx MISTRAL_ADMIN_COOKIE (security find-generic-password -s "mistral-admin-cookie" -w 2>/dev/null)
   end
   ```

   The `if not set -q` guard leaves an inherited value alone, which matters when
   a parent process already exported one.

3. Apply the dotfiles:

   ```bash
   dr
   ```

4. Open a new shell and run `usage-watch`.

Already wired this way: `mistral-api-key` → `MISTRAL_API_KEY` and `opencode-go`
→ `OPENCODE_API_KEY`. Add `mistral-admin-cookie` next to them.

## Troubleshooting

| Output                                         | Cause                                  | Fix                                            |
| ---------------------------------------------- | -------------------------------------- | ---------------------------------------------- |
| `set MISTRAL_ADMIN_COOKIE ... to fetch`        | cookie not set                         | follow [Cookie](#cookie)                       |
| `usage failed: too many redirects`             | session expired                        | [re-copy the cookie](#when-the-cookie-expires) |
| `HTTP 401` from `billing/v2/usage`             | session expired or wrong account       | re-copy the cookie while signed in             |
| `MISTRAL_ADMIN_COOKIE has no csrftoken cookie` | header copied without `csrftoken`      | copy the whole `Cookie` header                 |
| `whoami failed: HTTP 401`                      | bad `VIBE_API_KEY` / `MISTRAL_API_KEY` | create a new key                               |
| `HTTP 401 ... api_key_expired`                 | OpenCode key revoked or plan lapsed    | create a new key                               |
| `Could not parse your authentication token`    | stale ChatGPT access token             | re-read `~/.codex/auth.json`                   |
| `no provider configured`                       | no credential variables set            | set at least one from the table above          |
