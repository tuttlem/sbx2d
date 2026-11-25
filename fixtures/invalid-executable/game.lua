-- Pack files are data, never executable Lua.
return {
  pack_version = 1,
  name = "invalid-executable",
  start_scene = function() return "forest:entrance" end,
}
