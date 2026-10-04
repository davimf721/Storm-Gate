# Patch queues

One directory per component in `manifests/sources.lock`. Each has a `series`
file listing patches in application order. All queues are currently empty:
patches are added only with a reproduction, as described in
[docs/development/patches.md](../docs/development/patches.md).

Every patch must begin with this header (checked by `scripts/check-patches.sh`):

```text
Storm-Gate-Patch: SG-<COMPONENT>-NNNN
Problem: <what breaks, observable symptom>
Justification: <why it cannot be fixed elsewhere / why now>
Test: <synthetic test or reproduction that fails without it>
Upstream-Issue: <URL or "none yet">
Upstream-Status: not-submitted | submitted | accepted | rejected | not-applicable
```

Patches are licensed under the license of the component they modify.
