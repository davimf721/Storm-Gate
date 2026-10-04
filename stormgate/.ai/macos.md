# macOS agent

**Owns:** Cocoa / Objective-C bridges, Rosetta detection, GameController.framework
(`crates/input`), macOS filesystem specifics, code signing, notarization,
the future SwiftUI app.

Rules:
- Rosetta is an OS dependency: detect it, show the install command, never
  accept Apple's license on the user's behalf.
- Objective-C only where an Apple framework requires it; keep logic in Rust.
- The GUI (Phase 10) only calls existing Rust APIs; no exclusive logic in it.
- Distribution: ad-hoc signed first; Developer ID + notarization later.
- Track behaviour per macOS version (14, 15, 26, ...) in
  `docs/compatibility/matrix.md`.
