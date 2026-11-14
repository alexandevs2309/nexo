---
name: basilisco-backends
description: Work on BASILISCO native and WebAssembly backends, capability gates, code generation, and execution parity.
---

# Basilisco Backends

Establish actual supported subset before edits. Unsupported MIR operations must fail with explicit diagnostics; never silently become zero, null, success, or no-op. Test projections, indexing, overflow, division, unreachable paths, calls, globals, and initialization as relevant. Check installed targets/toolchains, not merely known target names. Compare behavior against reference interpreter/VM. Do not mislabel source generation as direct machine-code generation.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
