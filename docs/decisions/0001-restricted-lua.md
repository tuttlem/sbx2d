# ADR 0001: Parse a restricted Lua-shaped data format directly

## Status

Accepted for Milestone 0.

## Decision

`game.lua` accepts only `return { ... }` data literals and is parsed by
`sbx2d-pack`. No Lua VM or standard library is embedded.

## Consequences

Packs cannot run callbacks, loops, imports, filesystem operations, process
operations, or arbitrary expressions. The parser can produce source-aware
diagnostics without executing untrusted content. Compatibility with arbitrary
Lua tables is intentionally not a goal; broader data syntax must be explicitly
specified and tested before being accepted.
