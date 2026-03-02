# Run UXC Daemon as a Managed Service

This guide shows how to run `uxc` daemon under a service manager so runtime auth
environment (for example `OP_SERVICE_ACCOUNT_TOKEN`) is stable across requests.

Why this matters:

- Endpoint calls are handled by daemon.
- `--secret-op` is resolved at request runtime in daemon execution path.
- If daemon starts without 1Password auth context, `op://...` resolution fails.

Use foreground daemon entrypoint:

```bash
uxc daemon _serve
```

`uxc daemon start` is auto-spawn mode and is not the recommended `systemd/launchd` entrypoint.

## Linux (`systemd`)

1. Create service user (optional but recommended):

```bash
sudo useradd --system --home /var/lib/uxc --create-home --shell /usr/sbin/nologin uxc
```

2. Put token in root-owned env file:

```bash
sudo install -d -m 0750 /etc/uxc
sudo sh -c "cat >/etc/uxc/daemon.env" <<'EOF'
OP_SERVICE_ACCOUNT_TOKEN=ops_xxx
RUST_LOG=info
EOF
sudo chmod 0640 /etc/uxc/daemon.env
sudo chown root:uxc /etc/uxc/daemon.env
```

3. Create unit file `/etc/systemd/system/uxc-daemon.service`:

```ini
[Unit]
Description=UXC Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=uxc
Group=uxc
EnvironmentFile=/etc/uxc/daemon.env
ExecStart=/usr/local/bin/uxc daemon _serve
Restart=on-failure
RestartSec=2s
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

4. Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now uxc-daemon
sudo systemctl status uxc-daemon
```

5. Validate:

```bash
uxc daemon status
journalctl -u uxc-daemon -f
```

## macOS (`launchd`)

Create `~/Library/LaunchAgents/com.holon.uxc.daemon.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.holon.uxc.daemon</string>

  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/uxc</string>
    <string>daemon</string>
    <string>_serve</string>
  </array>

  <key>EnvironmentVariables</key>
  <dict>
    <key>OP_SERVICE_ACCOUNT_TOKEN</key>
    <string>ops_xxx</string>
    <key>RUST_LOG</key>
    <string>info</string>
  </dict>

  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>

  <key>StandardOutPath</key>
  <string>/tmp/uxc-daemon.out.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/uxc-daemon.err.log</string>
</dict>
</plist>
```

Load and start:

```bash
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.holon.uxc.daemon.plist
launchctl kickstart -k gui/$(id -u)/com.holon.uxc.daemon
uxc daemon status
```

## Token Rotation

After rotating `OP_SERVICE_ACCOUNT_TOKEN`, restart service:

```bash
# systemd
sudo systemctl restart uxc-daemon

# launchd
launchctl kickstart -k gui/$(id -u)/com.holon.uxc.daemon
```

## Security Notes

- Use dedicated 1Password Service Account with least privilege.
- Limit vault access (for example only `agents` vault, `read_items`).
- Do not commit tokens into repository files.
- Prefer service manager secret injection mechanisms over shell profiles.

## Related Docs

- 1Password secret sources and troubleshooting: [`auth-secret-sources.md`](auth-secret-sources.md)
- Logging and diagnostics: [`logging.md`](logging.md)
