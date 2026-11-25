# Fixed-step physics shell

Milestone 3 starts with an sbx2d-owned deterministic physics contract. A
`PhysicsWorld` advances dynamic bodies only in fixed `fixed_dt` increments,
applies gravity, filters collisions by layer/mask, reports contacts, and
exposes interpolated positions for rendering. Sensors report overlap without
solid resolution. The contract intentionally keeps solver state private to the
runtime so a future backend can replace the simple AABB implementation without
changing pack-facing semantics.
