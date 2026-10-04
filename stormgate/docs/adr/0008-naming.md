# ADR 0008: Naming and trademarks

## Context
"Proton" and "Steam" are Valve trademarks. Code may be reused under its
licenses, but trademarks may not be used as the product name.

## Decision
- Product: **Storm Gate**; CLI: `stormgate`; data directory: `StormGate`;
  environment variables: `STORMGATE_*`.
- Description: "An open-source Windows game compatibility runtime for macOS,
  built using Wine and technologies from the Proton ecosystem."
- Never "Proton for macOS" or "official Proton".

## Consequences
- Before a public release, check the name for conflicts with existing
  trademarks in the games space (there is a game titled "Stormgate") and
  rename early if needed; the name is isolated in `PRODUCT_NAME` and docs.
