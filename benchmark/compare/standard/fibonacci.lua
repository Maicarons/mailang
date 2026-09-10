function fibonacci(n)
    if n <= 1 then return n end
    return fibonacci(n - 1) + fibonacci(n - 2)
end

_ = fibonacci(10)
local start = os.clock()
local result = fibonacci(30)
local elapsed = (os.clock() - start) * 1000
print(string.format("fibonacci(30) = %d", result))
print(string.format("elapsed_ms=%.2f", elapsed))
