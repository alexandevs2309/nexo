---
name: basilisco-runtime-vm
description: Work on BASILISCO runtime values, interpreter, VM, memory semantics, closures, tasks, channels, and execution errors.
---

# Basilisco Runtime Vm

Treat Tree-Walk and VM as implementations of one language contract. For changed semantics add tests for both engines where supported. Check values, equality, errors, side effects, evaluation order, short-circuiting, collections, closures/captures, scheduling, channels, and resource limits. Do not silently equate Copy and Move or hide runtime errors. Concurrency changes need deterministic tests and explicit limitations.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
