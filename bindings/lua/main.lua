-- Lua demo for MaìLang
local mailang = require("mailang")

local interp = mailang.create()

local result = interp:eval("1 + 2")
print("Result: " .. result)

interp:destroy()
