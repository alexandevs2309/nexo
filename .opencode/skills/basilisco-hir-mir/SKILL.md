---
name: basilisco-hir-mir
description: Work on BASILISCO HIR/MIR lowering, invariants, verification, places, operands, and control flow.
---

# Basilisco Hir Mir

Inspect definitions, lowering, verifier, and consumers. Preserve source spans and type information. Verify block targets, terminators, locals, places/projections, operand types, call signatures, initialization/dataflow, and control-flow invariants. Add malformed-IR tests; verifiers must reject invalid IR rather than relying on backends. Preserve shadowing and name-resolution semantics.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
