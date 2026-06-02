local M = {}

function M.add(a, b)
  return a + b
end

local function helper(x)
  return x * 2
end

M.banner = "[…CTY]"

return M
