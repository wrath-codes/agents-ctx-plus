-- Lua sample fixture for rich extraction
local M = {}

-- Adds two numbers.
function M.add(a, b)
    return a + b
end

-- Greets with receiver syntax.
function M:greet(name)
    return "hello " .. name
end

-- Local helper function.
local function helper(x)
    return x * 2
end

local answer<const> = 42
local temp<close> = io.open("data.txt", "r")

local Config = {
    enabled = true,
    build = function(x)
        return x
    end,
    ["mode"] = "fast",
}

M.version = "1.0"
M.scale = function(v)
    return v * 10
end
M["alias"] = function(v)
    return v
end

GlobalTable = {
    ping = function(v)
        return v
    end,
    level = 1,
}

global_counter = 0

return M
