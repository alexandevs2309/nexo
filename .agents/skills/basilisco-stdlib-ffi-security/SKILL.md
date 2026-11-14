---
name: basilisco-stdlib-ffi-security
description: Work on BASILISCO standard library, FFI, dynamic loading, unsafe boundaries, and runtime security.
---

# Basilisco Stdlib Ffi Security

Inspect duplicated builtin implementations and signatures/effects. Treat ABI, pointers, ownership, lifetimes, symbols, return types, and dynamic loading as unsafe boundaries. Never claim sandboxing unless implemented and tested. External libraries execute native code in-process. Test invalid symbols, error propagation, and safe-to-test boundary failures. Do not broaden unsafe surface without approval and a threat model.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
