# ADR 0001: Rust for orchestration

## Context
The runtime needs a CLI, process management, configuration, prefix
management, downloads, hashing, Steam library parsing, logs and an updater.
Wine and the graphics layers are C/C++ and stay that way.

## Decision
All orchestration is Rust, organised as one crate per subsystem in
`crates/`. C/C++ only inside Wine/DXMT/DXVK/VKD3D/bridges. Objective-C only
where Apple frameworks require it. Swift/SwiftUI is reserved for the GUI,
which only calls the Rust core.

## Consequences
- Memory-safe parsing of untrusted inputs (PE files, VDF, archives).
- The core builds and is tested on Linux CI; only runtime components need a Mac.
- The GUI cannot hold exclusive logic.
