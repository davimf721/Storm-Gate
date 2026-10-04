# ADR 0004: Vulkan-on-Metal is abstracted

## Context
DXVK and VKD3D-Proton need Vulkan. On macOS that means MoltenVK (mature,
Apache-2.0, works before Metal 4) or Mesa's KosmicKrisp (Metal 4, newer
macOS). Both implement a portability subset.

## Decision
`VulkanDriver { MoltenVk, KosmicKrisp }` in `crates/config`; the runtime
ships each driver's ICD and the launcher points `VK_ICD_FILENAMES` at the
chosen one. MoltenVK is the default; KosmicKrisp is optional.

## Consequences
- Profiles can pick a driver per game.
- Features are probed, never assumed from Linux behaviour.
