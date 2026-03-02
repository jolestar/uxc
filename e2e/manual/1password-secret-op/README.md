# Manual E2E: 1Password Secret Source (`--secret-op`) Runtime Resolution

## Goal

Verify that credentials configured with `--secret-op` are resolved at runtime (during request execution), not at `credential set` time.

## Prerequisites

- `uxc` built locally (`target/debug/uxc`).
- `op` CLI installed and available in `PATH`.
- For success path: at least one valid 1Password account/session (`op signin` completed).

## Test Data

- Credential ID: `op-e2e`
- Example secret reference: `op://Engineering/demo/token`
- Endpoint used to trigger runtime auth resolution: `https://petstore3.swagger.io/api/v3/openapi.json`

## Case A: No `op` login/session (expected failure)

1. Stop daemon to avoid stale daemon state:

```bash
target/debug/uxc daemon stop
```

2. Configure credential with `--secret-op`:

```bash
target/debug/uxc auth credential set op-e2e \
  --auth-type bearer \
  --secret-op op://Engineering/demo/token
```

3. Trigger a request execution path:

```bash
target/debug/uxc https://petstore3.swagger.io/api/v3/openapi.json -h --auth op-e2e
```

Expected:
- Command fails.
- Error message contains `Failed to resolve 1Password secret for credential 'op-e2e'`.
- This confirms runtime calls trigger `op read ...`.

## Case B: Logged-in `op` session (expected success)

1. Ensure active `op` session:

```bash
op whoami
```

2. Re-run the same request:

```bash
target/debug/uxc https://petstore3.swagger.io/api/v3/openapi.json -h --auth op-e2e
```

Expected:
- No `Failed to resolve 1Password secret ...` error.
- Request proceeds to endpoint/protocol execution stage.

## Optional Case C: Mock `op` for deterministic local verification

Use this when real 1Password data/session is unavailable.

1. Create a mock `op` binary:

```bash
TMP_DIR="$(mktemp -d)"
mkdir -p "$TMP_DIR/bin"
cat > "$TMP_DIR/bin/op" <<'EOF'
#!/bin/sh
if [ "$1" = "read" ] && [ "$2" = "op://Engineering/demo/token" ]; then
  printf "token-from-mock-op"
  exit 0
fi
echo "unexpected args: $*" >&2
exit 1
EOF
chmod +x "$TMP_DIR/bin/op"
```

2. Run with mock binary first in `PATH`:

```bash
PATH="$TMP_DIR/bin:$PATH" target/debug/uxc https://petstore3.swagger.io/api/v3/openapi.json -h --auth op-e2e
```

Expected:
- No 1Password resolution error.
- Confirms `op read` invocation path works.

## Cleanup

```bash
target/debug/uxc auth credential remove op-e2e
```
