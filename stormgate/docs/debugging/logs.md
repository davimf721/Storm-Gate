# Logs and diagnostics

Every launch creates a session directory:

```text
logs/<date>/<label>-<HHMMSS>/
├── launcher.log   decisions: runtime, prefix, graphics per API, deployed DLLs, exact command, exit code
├── wine.log       Wine stdout/stderr
├── wineboot.log   prefix creation/repair, when it happened
├── steam.log      Steam client / installer
├── graphics/      DXVK/DXMT/VKD3D logs (--debug-graphics)
└── system.json    chip, macOS, memory, Rosetta, Storm Gate version
```

Labels: `app-<appid>` (Steam games), `run-<exe>`, `test-<name>`,
`prefix-<name>`, `steam`, `steam-install`.

## Commands

```bash
stormgate logs last                 # newest session + tail of wine.log
stormgate logs last 1091500         # newest session for an AppID
stormgate logs bundle 1091500       # stormgate-report-app-1091500.zip
stormgate --dry-run game run 1091500   # print the command without running it
```

## Redaction

Bundles replace: your home directory (`~`), your user name (`<user>`),
credential-like `key=value` pairs (token, password, secret, cookie, session,
auth, api key), `Bearer`/`Basic` headers, `steamLoginSecure`, 64-bit SteamIDs
and e-mail addresses. Binary files are skipped. Redaction is best effort —
review bundles before posting them publicly.

## Wine debug channels

Normal runs use `WINEDEBUG=-all,err+all`. `--debug-graphics` switches to
`+loaddll,+seh,+module,err+all,warn+all`. For anything else, use
`stormgate prefix shell <name>` and run Wine by hand with your own
`WINEDEBUG`.
