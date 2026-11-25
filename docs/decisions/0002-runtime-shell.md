# Runtime shell backend

Status: accepted for Milestone 2 shell work.

The runtime owns platform-neutral scene lifecycle, capability registration,
asset loading, transforms, sprites, cameras, and render-frame extraction.
Window and GPU integration remain an adapter around those contracts. This keeps
pack validation and headless tests independent of a display server while
allowing a future winit-based adapter to provide the cross-platform event loop.

The first adapter will target winit for window/input lifecycle and a small
renderer backend selected by a separate spike. The `sbx2d run` command currently
executes the lifecycle headlessly; graphical execution must not change scene
ownership or render ordering semantics.
