# Getting started

> Storm Gate is pre-alpha. Expect breakage and please report it.

## 1. Check your Mac

```bash
stormgate doctor
```

You need an Apple Silicon Mac (M1 or newer), macOS 14 or newer and Rosetta 2.
If Rosetta is missing, `doctor` prints the command; run it yourself (it
accepts Apple's license):

```bash
/usr/sbin/softwareupdate --install-rosetta --agree-to-license
```

## 2. Install a runtime

Until binary releases exist, build one (see [building.md](building.md)):

```bash
make runtime && make install-runtime
stormgate runtime list
```

Runtimes install side by side. Switch or roll back at any time:

```bash
stormgate runtime use 0.0.1
```

## 3. Run a Windows program

```bash
stormgate run ~/Games/MyGame/Game.exe
stormgate run --backend dxvk ~/Games/MyGame/Game.exe      # force a backend
stormgate run --debug-graphics ~/Games/MyGame/Game.exe    # verbose logs
stormgate graphics probe ~/Games/MyGame/Game.exe          # what would be used
```

Each folder gets its own prefix (`--prefix name` to choose). Manage them with
`stormgate prefix list|inspect|clone|repair|delete|shell`.

## 4. Steam

```bash
stormgate steam install     # creates the `steam` prefix, installs SteamSetup.exe
stormgate steam start       # log in inside the Steam window
stormgate steam library     # games installed in the prefix
stormgate game add 1091500  # detect and (after you confirm) save a profile
stormgate game run 1091500
```

Storm Gate never sees your Steam credentials.

## 5. When something breaks

```bash
stormgate logs last 1091500
stormgate logs bundle 1091500     # sanitized zip for a bug report
```

See [troubleshooting.md](troubleshooting.md).
