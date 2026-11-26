# sbx2d roadmap

## 1. Product statement

`sbx2d` is a native Rust 2D game runtime that interprets a **declarative game
pack**. A pack describes what exists and which native capabilities compose it:
assets, worlds, scenes, entity templates, physics properties, input mappings,
animations, cameras, transitions, events, conditions, and actions.

The pack author does not implement algorithms, write callbacks, or execute
arbitrary Lua at runtime. Rust owns all mechanics, performance-sensitive work,
and lifecycle control. The engine is the interpreter; Lua is an author-friendly
data notation used only at the pack boundary.

The design goal is a generic, composable 2D foundation rather than a platformer
engine with generic words painted over it. "Player", "enemy", "platform", and
"projectile" are game-pack compositions, not engine types.

## 2. Non-negotiable principles

1. **Rust owns mechanics.** Physics, collision, timing, rendering, animation,
   input capture, camera transforms, audio mixing, loading, and entity lifecycle
   are native systems.
2. **Packs are declarative.** Lua expresses tables and scalar values only; it is
   not a gameplay language. No user function, callback, loop, module import,
   filesystem access, process access, or arbitrary evaluation is permitted.
3. **Capabilities are native and schema-driven.** Every component, behaviour,
   event, condition, action, controller, and asset type has a stable name,
   validation schema, runtime implementation, and documented constraints.
4. **Composition is the extension mechanism.** A game combines small native
   capabilities. When composition cannot express a new verb, add a reusable
   Rust capability—not an escape hatch for imperative scripts.
5. **Validation precedes execution.** A complete pack can be discovered,
   resolved, schema-validated, and diagnosed without opening a window, loading
   GPU resources, or simulating a world.
6. **Determinism is designed, not promised casually.** Fixed simulation steps,
   explicit ordering, seeded randomness, and stable identifiers are prerequisites
   for replay/testing. Exact cross-platform determinism is a later, explicit
   compatibility target.
7. **Content is portable.** Game packs are independent of platform code; the
   runtime targets Windows, Linux, and macOS.

## 3. Responsibility boundary

| Capability | Rust runtime owns | Game pack configures |
| --- | --- | --- |
| Physics | fixed-step integration, broad/narrow phase, contacts, resolution | gravity, timestep policy, materials, layers, bodies, colliders |
| Movement | conversion of controller intent to forces/velocity/impulses | controller choice and parameters |
| Rendering | window/GPU lifecycle, batching, transforms, draw order | sprites, layers, colours, visibility, parallax |
| Animation | time advancement, frame sampling, transition evaluation | clips, frame timing, state/trigger mappings |
| Input | device events and action-state production | action names and device bindings |
| Camera | transforms, viewport handling, follow calculation | follow target, zoom, bounds, parallax |
| Audio | decoding, mixing, channels, spatial policy | sound/music assets and trigger bindings |
| Gameplay | event dispatch, condition evaluation, action execution | entity composition and declarative rules |
| Scenes | loading/unloading and persistence boundaries | world/scene contents and transitions |

Lua must never implement a physics solver, execute event handlers, or advance a
simulation tick. A pack may request native operations such as an impulse or a
scene transition only through validated actions.

## 4. Pack model and layout

The first pack format uses Lua files whose returned values are data tables. The
loader parses a restricted subset, reports source locations, and converts the
tables into Rust-owned typed definitions. It must reject functions, metatables,
dynamic loading, and side-effecting standard libraries.

```text
my-game/
  game.lua                         # manifest, defaults, start scene
  common/
    assets/
    entities/
    animations/
    audio/
    input.lua
  worlds/
    forest/
      world.lua                    # world overrides/defaults
      assets/
      scenes/
        entrance/scene.lua
        river/scene.lua
    caves/
      world.lua
      scenes/
        entrance/scene.lua
        boss/scene.lua
```

Definitions receive qualified identifiers, e.g. `common:hero`, `forest:oak`,
or `caves:bat`. References are explicit and never resolved by filename guessing.

Configuration resolution is predictable:

```text
engine defaults → game defaults → world defaults → scene defaults
                → entity template → entity instance override
```

The merge rules are type-specific and documented: scalar replacement, named-map
merge, explicit list replacement or append syntax, and no implicit deep merge
where its meaning would be ambiguous. Resolved definitions are immutable input
to world instantiation.

## 5. Worlds, scenes, and persistence

- A **game** owns metadata, shared defaults, input mappings, global progression,
  and an initial scene reference.
- A **world** groups related scenes, shared environmental/physics defaults, and
  local assets.
- A **scene** is loadable content: backgrounds, terrain, cameras, entity
  instances, spawns, transitions, and scene-local rules.
- An **entity template** is reusable configured composition; an **entity
  instance** supplies placement and limited overrides.

Initial runtime scope: one active gameplay scene plus persistent game-level
state. Transitions unload scene-local entities, preserve explicitly persistent
state, create the destination scene, then select a named spawn point.

Concurrent scenes are a planned extension for persistent HUDs, background
layers, split screen, and seamless streaming. Do not make the first scene API
depend on that feature, but keep scene ownership and render routing explicit so
it can be added without a rewrite.

## 6. Native capability model

Each capability registration provides:

- a stable, namespaced identifier, such as `physics.body` or
  `movement.platformer`;
- a versioned configuration schema and defaults;
- required and produced components/resources;
- validation rules, including incompatibilities and reference requirements;
- lifecycle hooks for construction, update, events, and teardown;
- user-facing documentation and diagnostic identifiers.

### First capability families

**Foundation:** transform, hierarchy/ownership, tags, lifetime, visibility,
named spawn points, timers, deterministic random streams.

**Physics:** static, dynamic, and kinematic bodies; rectangle/circle/capsule
colliders; collision layers/masks; sensors; material friction/restitution;
gravity scale; contact events; fixed-step simulation; debug drawing.

**Controllers:** platformer, top-down, patrol, follow, orbit, constant velocity,
oscillation, and simple projectile motion. Controllers turn declarative intent
and input/action state into native physical commands.

**Presentation:** sprite, tile layer, animated sprite, render layer/order,
colour/tint, parallax background, camera, camera follow, bounds, and screen
effects.

**Interaction:** health, damage, area effects, collectables, inventory values,
score, cooldowns, timers, spawn/despawn, doors/transitions, and checkpoints.

**Media:** sound/music playback, loops, volume groups, animation clips and
native animation-state transitions.

## 7. Physics contract

Physics runs entirely on the Rust side with a fixed timestep independent of
render rate. A scene/world may select from safe runtime policies, but the pack
does not advance the solver.

Initial configurable properties include gravity vector, body type, gravity
scale, mass/density, friction, restitution, collision layer/mask, shape,
sensor state, and continuous-collision options where supported.

The event/action interpreter exposes bounded physics commands, such as
`physics.apply_force`, `physics.apply_impulse`, `physics.set_velocity`, and
`physics.set_gravity_scale`, plus facts such as contact began/ended and
grounded state. These commands are queued and applied in a documented phase;
they never expose solver internals or mutable physics structures to the pack.

The initial technical evaluation compares Bevy ECS + Avian2D against a smaller
custom ECS/physics composition. Selection criteria: stable API, 2D collision
features, fixed-step integration, debug tooling, platform support, maintenance
cost, and the ability to preserve sbx2d's own pack-facing contracts.

## 8. Events, conditions, and actions

Events provide the declarative alternative to callbacks. They are typed facts
emitted by native systems, e.g. `physics.contact_started`, `health.depleted`,
`input.action_pressed`, `timer.elapsed`, `animation.finished`, or
`scene.entered`.

Rules are data:

```lua
{
  when = "health.depleted",
  if_all = {
    { fact = "entity.has_tag", value = "breakable" }
  },
  actions = {
    { type = "animation.play", clip = "explode" },
    { type = "audio.play", sound = "common:boom" },
    { type = "area.damage", radius = 100, amount = 50 },
    { type = "entity.destroy" }
  }
}
```

Rules need a formal execution model before gameplay capability growth:

- event phases and producer ordering;
- target/context binding (`self`, instigator, contact other, named references);
- condition syntax with boolean composition and comparisons;
- action queues, ordering, and conflict semantics;
- recursion/loop limits and a per-tick action budget;
- failures as diagnostics rather than silent no-ops;
- deterministic ordering where required.

No embedded expression language belongs in the first release. Begin with typed,
schema-validated conditions and action arguments. Add declarative value sources
only after concrete pack needs prove the minimal vocabulary insufficient.

## 9. Runtime phases

The intended frame model is:

```text
platform input → input action state → fixed simulation accumulator
  └─ for each fixed tick:
       native pre-physics controllers
       queued declarative actions
       physics simulation
       contact/event collection
       event → condition → action interpretation
       entity lifecycle commit
render interpolation → rendering → audio presentation
```

Precise ordering, especially for actions generated during a tick, becomes a
tested public semantic contract. Rendering never changes simulation state.

## 10. Tooling and diagnostics

The CLI is a first-class pack-authoring tool:

```text
sbx2d validate ./my-game
sbx2d list-worlds ./my-game
sbx2d list-scenes ./my-game
sbx2d inspect entity common:hero --pack ./my-game
sbx2d run ./my-game
sbx2d run ./my-game --scene forest/river
```

Required diagnostic quality: file/line/field path, expected versus received
shape/value, full qualified reference, inheritance source, and actionable
remediation. Later tools include schema export, capability documentation,
pack formatting, asset checks, visual physics inspection, event tracing, replay,
and a headless test runner.

## 11. Workspace layout

```text
sbx2d/
  crates/
    sbx2d-pack/       # loading, typed definitions, resolution, validation
    sbx2d-runtime/    # native systems and capability registry
    sbx2d-cli/        # validate/inspect/run commands
  examples/
    first-platformer/ # canonical vertical-slice game pack
  docs/
  ROADMAP.md
```

The `sbx2d-pack` crate must not depend on the renderer, windowing, or physics
backend. `sbx2d-runtime` consumes validated pack definitions behind narrow
interfaces. `sbx2d-cli` composes both.

## 12. Delivery plan

### Milestone 0 — Foundation and contracts ✅

- Establish workspace conventions, CI, formatting, linting, error/reporting
  policy, supported Rust toolchain, and license.
- Write the pack specification, identifier/reference grammar, configuration
  merge rules, and initial schema model.
- Build a minimal game manifest model plus source-aware diagnostics.
- Decide and document the restricted-Lua loading approach; prove rejection of
  executable constructs.

**Exit criterion:** `sbx2d validate` accepts/rejects simple manifests with
excellent errors and requires no windowing dependency.

### Milestone 1 — Pack discovery and resolution ✅

- Discover `game.lua`, `common/`, worlds, scenes, and qualified definitions.
- Resolve references and inheritance in a reproducible order.
- Validate assets exist and no identifiers/references collide.
- Implement `list-worlds`, `list-scenes`, and `inspect`.

**Exit criterion:** a multi-world pack validates fully and produces a resolved,
serialisable definition graph.

### Milestone 2 — Runtime shell ✅

- Choose/window backend and create cross-platform window/input loop.
- Add runtime capability registry, lifecycle, scene instantiation, transforms,
  sprites, camera, and basic asset loading.
- Implement `run` for one active scene.

**Exit criterion:** a configured scene opens, renders a camera-visible sprite,
and exits cleanly on all supported development platforms.

### Milestone 3 — Fixed-step 2D physics ✅

- Integrate the selected physics backend behind sbx2d-owned abstractions.
- Add gravity, bodies, colliders, collision layers, sensors, contacts, and
  debug drawing.
- Add fixed timestep/accumulator, interpolation policy, and deterministic test
  fixtures.

**Exit criterion:** a declaratively defined falling body collides reliably with
a configured floor at varied rendering frame rates.

### Milestone 4 — Declarative interaction engine ✅

- Introduce typed events, conditions, action schemas, queues, and limits.
- Implement health/damage, sound trigger, spawn/despawn, physics commands, and
  scene transition actions.
- Add an event/action trace suitable for debugging packs.

**Exit criterion:** a configured crate can take contact damage, play an effect,
damage an area, update score, and despawn without author code.

### Milestone 5 — First playable vertical slice

- Add input mappings, platformer controller, grounded detection, jump rules,
  animated sprites, camera follow, collectables, patrol enemy, and transitions.
- Create `examples/first-platformer` as executable specification and regression
  fixture.

**Exit criterion:** the example is a small complete multi-scene platform game
defined solely by pack data.

### Milestone 6 — Authoring hardening

- Improve diagnostics, migration/versioning rules, documentation generation,
  headless validation, asset linting, automated pack tests, and replay hooks.
- Establish compatibility policy for capability schemas and pack versions.

**Exit criterion:** a new pack author can create and diagnose a small game from
documentation without changing Rust.

### Milestone 7 — Breadth after proof

- Top-down/vehicle controllers, tilemaps, richer camera modes, inventory,
  dialogue as declarative data, particle effects, save/load, accessibility,
  controller rebinding, and concurrent scene support.

Each addition must earn its place through a pack-level use case and remain a
native capability with a clear contract.

## 13. Decisions to make before implementation depth

1. Rust MSRV, edition policy, licence, and CI targets.
2. Restricted Lua parser/loader strategy and whether it accepts only `return
   { ... }` data literals.
3. ECS/rendering/physics stack after small throwaway spikes—not based on
   ecosystem popularity alone.
4. Coordinate system, units, pixel-to-world convention, and gravity sign.
5. Schema/pack versioning and compatibility/migration policy.
6. Asset identifiers, import formats, loading ownership, and hot-reload scope.
7. Exact configuration merge semantics, especially arrays/lists.
8. Event phase/action ordering, recursive-event handling, and action budgets.
9. Persistence model for game/world/scene/entity state and save-file boundary.
10. Testing/replay determinism requirements and whether network play is in
    scope at all.

## 14. Explicit non-goals for the first vertical slice

- General-purpose Lua gameplay scripting or any arbitrary code execution.
- 3D rendering/physics.
- Network multiplayer.
- User-editable visual editor.
- Seamless streaming or multiple concurrent gameplay scenes.
- Universal genre coverage or every desired behaviour.
- A public plugin ABI.

## 15. Immediate next work

1. Turn Milestone 0 into a concise, testable pack-format specification.
2. Define the initial Rust data structures and source-aware error types in
   `sbx2d-pack`.
3. Create two tiny fixture packs: one valid, one intentionally invalid.
4. Implement `sbx2d validate` before choosing Bevy, Avian2D, or renderer APIs.
5. Add architecture decision records as the loader and runtime choices are made.

The first proof is deliberately small: a pack validates; a scene resolves; a
sprite with a dynamic physics body falls onto a static floor; an input-configured
platformer controller moves and jumps; an event rule triggers a native action.
Only then expand breadth.

