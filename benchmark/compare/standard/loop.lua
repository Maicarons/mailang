local start = os.clock()
local s = 0
for i = 0, 99999 do
    s = s + i
end
local elapsed = (os.clock() - start) * 1000
print(string.format("sum = %d", s))
print(string.format("elapsed_ms=%.2f", elapsed))
