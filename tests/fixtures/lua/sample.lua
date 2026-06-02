-- Calculator module.
local M = {}

function M.add(a, b)
  return a + b
end

local function helper(x)
  return x * 2
end

M.banner = "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the lua fixture and here is some extra padding text appended to comfortably exceed the limit"

return M
