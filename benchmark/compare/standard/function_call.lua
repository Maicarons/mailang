function add(a, b) return a + b end
local start = os.clock()
local result = 0
for i = 0, 99999 do result = add(result, i) end
local elapsed = (os.clock() - start) * 1000
print(string.format("result = %d", result))
print(string.format("elapsed_ms=%.2f", elapsed))
