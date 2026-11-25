# Pack format version 1

An sbx2d pack is a directory containing `game.lua`. It is data, not a Lua
program: the complete file must be one `return { ... }` expression.

## Manifest

```lua
return {
  pack_version = 1,
  name = "my-game",
  start_scene = "forest:entrance",
}
```

`pack_version` is the positive integer format version and currently must be
`1`. `name` is the unqualified game identifier. `start_scene` is a qualified
reference to the initial scene and must resolve to a discovered scene.

Identifiers use lowercase ASCII: they begin with `a`–`z`, followed by zero or
more lowercase letters, digits, hyphens, or underscores. Qualified references
have precisely one colon: `namespace:name`.

The v1 manifest rejects unknown or duplicate fields. This gives authors a
reliable typo diagnostic while its schema is intentionally small.

## Restricted Lua data notation

The loader accepts only `return`, named table fields (`name = value`), strings,
base-10 signed integers, `true`, `false`, `nil`, commas/semicolons, and
single-line `--` comments. Nested tables are lexically accepted but do not yet
appear in the v1 manifest schema.

It rejects functions, calls, variables, operators, metatables, computed keys,
arrays, module loading, and any content after the returned table. A pack is
parsed directly into Rust-owned data; sbx2d does not link or invoke a Lua VM.

## Configuration merge model

Future definitions resolve in this order:

```text
engine defaults → game defaults → world defaults → scene defaults
                → entity template → entity instance override
```

Scalars replace. Named maps merge by name. Lists require an explicit future
schema choice between replacement and append; there is no implicit deep merge.
The v1 manifest has no inherited fields, so merge behavior is specified now but
not silently implemented before the definitions that need it exist.

## Discovery and resolution

Milestone 1 uses this deterministic layout:

```text
game.lua
common/**/definition.lua             # each has id = "..."
worlds/<world>/world.lua
worlds/<world>/scenes/<scene>/scene.lua
worlds/<world>/**/definition.lua     # outside scenes and world.lua
```

World and scene directory names are their identifiers. `name` is optional in a
world or scene file, but when present it must match its directory. A reusable
definition has a required `id` field and receives the namespace `common` or its
world directory: `common:hero`, `forest:oak`.

Files return data tables. The `extends` field may name a qualified definition
or scene. Resolution is deterministic and rejects missing parents and cycles.
Named tables merge recursively; scalar values and lists replace their parent
value. Asset declarations use `assets = { name = "relative/file.png" }`; every
declared asset must exist and is qualified with the enclosing namespace.

`sbx2d validate`, `list-worlds`, and `list-scenes` load the complete resolved
pack. `sbx2d inspect common:hero <pack>` prints the resolved definition as
canonical JSON.
