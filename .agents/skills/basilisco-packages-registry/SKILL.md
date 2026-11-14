---
name: basilisco-packages-registry
description: Work on BASILISCO manifests, dependency resolution, archives, lockfiles, registry API, signing, and publishing security.
---

# Basilisco Packages Registry

Trace recursive dependency resolution relative to each dependency root. Propagate manifest, hash, IO, signature, and persistence errors; do not replace failures with empty hashes/defaults. Preserve archive defenses: traversal, symlinks/special files, duplicate paths, size/count/compression limits, normalization, hashes, signatures. Review atomicity, reproducibility, auth, authorization, rate limits, request limits, audit logs, and TLS termination. Never publish without explicit approval.

Always follow root `AGENTS.md`. Work only within the explicitly authorized task and stop before any phase transition.
