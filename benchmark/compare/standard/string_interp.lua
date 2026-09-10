local start = os.clock()
local result = ""
for i = 0, 9999 do
    result = string.format("item_%d", i)
end
local elapsed = (os.clock() - start) * 1000
print(string.format("last = %s", result))
print(string.format("elapsed_ms=%.2f", elapsed))
