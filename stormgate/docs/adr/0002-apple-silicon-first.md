# ADR 0002: Apple Silicon first

## Context
Most current Macs are Apple Silicon; Intel Macs are a shrinking target.
Windows games are overwhelmingly x86/x86_64.

## Decision
Tier 1 is Apple Silicon (M1+) running Windows x86_64 (and 32-bit via new
WoW64) through **Wine x86_64 under Rosetta 2**. Windows ARM64/ARM64EC is
Tier 2 (after the MVP). Intel Macs are Tier 3 and must not block work.
Rosetta is an OS dependency: detected and explained, never redistributed,
and its license is never accepted on the user's behalf.

## Consequences
- Wine and every library it loads (MoltenVK, freetype...) need x86_64 slices.
- CPU translation is isolated behind the runtime so alternatives (e.g. FEX,
  native ARM64EC Wine) can be added later.
