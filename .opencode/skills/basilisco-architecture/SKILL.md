---
name: basilisco-architecture
description: Map and evolve BASILISCO's Rust workspace architecture, crate boundaries, dependencies, and public interfaces.
---

# Basilisco Architecture

Map the current workspace, modules, dependency direction, features, and public APIs before changing architecture. Trace call paths and downstream consumers. Avoid broad refactors for narrow fixes. Propose an ADR for crate splits, public API changes, dependency changes, or cross-cutting redesigns. Report evidence, impact, migration plan, and tests. Never assume old architecture notes are current.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
