---
name: basilisco-semantic-analysis
description: Work on BASILISCO name resolution, scopes, type checking, mutability, control-flow validation, and semantic diagnostics.
---

# Basilisco Semantic Analysis

Trace AST through semantic analysis into HIR. Test lexical scopes, shadowing, function/local name collisions, arity, argument/return types, mutability, initialization, iterable types, index/field assignments, and source locations as applicable. Do not make unknown types universally compatible without an explicit language contract. Add positive and negative tests with useful diagnostics.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
