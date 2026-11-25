# sbx2d

`sbx2d` is a Rust 2D game runtime that interprets declarative game packs. Game
authors configure worlds, scenes, entities, native behaviours, and event/action
rules; they do not write imperative gameplay scripts.

The vision, constraints, architecture, and incremental delivery plan live in
[ROADMAP.md](ROADMAP.md).

The supported minimum Rust version is 1.85.0. Run `cargo fmt --all`,
`cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test
--workspace` before contributing; CI runs the same checks.

## Workspace

- `sbx2d-pack`: game-pack discovery, restricted Lua data loading, validation,
  references, inheritance, and diagnostics.
- `sbx2d-runtime`: native runtime capabilities: ECS integration, fixed-step
  physics, animation, input, audio, cameras, scene lifecycle, and the
  event/action interpreter.
- `sbx2d-cli`: developer commands such as `validate`, `list-scenes`, and `run`.

`sbx2d run <pack>` starts the validated active scene headlessly. Use
`sbx2d run <pack> --window` to start the winit window/event-loop adapter; the
current shell reports the active scene and render-item count in the title while
the pixel renderer is being integrated.

This workspace is intentionally dependency-free until the first vertical slice
settles its public models and capability contracts.

