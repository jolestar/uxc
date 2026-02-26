# Manual E2E: GitHub MCP OAuth Full Flow

## Goal

Verify the full OAuth lifecycle for MCP HTTP using the new auth model (`credential + binding`, no profile):

1. OAuth device login succeeds
2. Binding auto-match works without `--auth`
3. MCP calls succeed
4. OAuth refresh succeeds
5. Logout invalidates runtime calls

## Prerequisites

- Built binary is available (`target/debug/uxc`), or replace with your installed `uxc`.
- Network access to `https://api.githubcopilot.com/mcp`.
- GitHub account that can authorize device flow.
- A valid OAuth client ID for GitHub device flow (example: GitHub CLI client ID).

## Test Data

- Credential ID: `gh-mcp-oauth`
- Binding ID: `github-copilot-mcp`
- Endpoint: `https://api.githubcopilot.com/mcp`

## Steps

1. Start OAuth login:

```bash
target/debug/uxc auth oauth login gh-mcp-oauth \
  --endpoint https://api.githubcopilot.com/mcp \
  --flow device_code \
  --client-id <GITHUB_CLIENT_ID> \
  --scope read:user
```

2. Open the printed URL and enter the printed user code, then approve authorization.

3. Verify OAuth credential is stored:

```bash
target/debug/uxc auth oauth info gh-mcp-oauth
```

Expected:
- `ok: true`
- `auth_type: "oauth"`
- `oauth.has_refresh_token: true`

4. Add endpoint binding:

```bash
target/debug/uxc auth binding add \
  --id github-copilot-mcp \
  --host api.githubcopilot.com \
  --path-prefix /mcp \
  --scheme https \
  --credential gh-mcp-oauth \
  --priority 100
```

5. Verify binding match:

```bash
target/debug/uxc auth binding match https://api.githubcopilot.com/mcp
```

Expected: `matched: true` and `credential: "gh-mcp-oauth"`.

6. Run MCP call without `--auth` (auto-match):

```bash
target/debug/uxc https://api.githubcopilot.com/mcp list
target/debug/uxc https://api.githubcopilot.com/mcp get_me
```

Expected:
- `list` returns `ok: true` with tool list
- `get_me` returns `ok: true` with authenticated user data

7. Run manual refresh:

```bash
target/debug/uxc auth oauth refresh gh-mcp-oauth
target/debug/uxc auth oauth info gh-mcp-oauth
```

Expected:
- Refresh returns `ok: true`
- Token/expires data updated

8. Logout and verify credential is no longer usable:

```bash
target/debug/uxc auth oauth logout gh-mcp-oauth
target/debug/uxc https://api.githubcopilot.com/mcp get_me
```

Expected:
- Logout returns `ok: true`
- `get_me` fails with missing OAuth access token error

## Cleanup

```bash
target/debug/uxc auth binding remove github-copilot-mcp
target/debug/uxc auth credential remove gh-mcp-oauth
target/debug/uxc auth credential list
target/debug/uxc auth binding list
```

Expected:
- Test credential and binding are removed
- Lists do not include `gh-mcp-oauth` / `github-copilot-mcp`

## Notes

- Do not run `list/get_me/refresh` in parallel while validating behavior; parallel runs may cause refresh race noise across processes.
- If login does not complete in time, rerun from step 1 to get a new device code.

