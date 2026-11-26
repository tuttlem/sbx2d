return {
  sprite = "common:hero",
  bodies = {
    { id = "hero", type = "dynamic", x = 480, y = 50, width = 12, height = 12, friction = 0.6, restitution = 0.15 },
    { id = "crate", type = "static", x = 480, y = 456, width = 12, height = 12 },
    { id = "floor", type = "static", x = 480, y = 480, width = 480, height = 12 },
  },
  health = { id = "crate", health = 1 },
  on_contact = { first = "hero", second = "crate", target = "crate", damage = 1, score = 100, sound = "crate-hit" },
}
