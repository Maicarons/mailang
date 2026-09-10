import time

def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

_ = fibonacci(10)
start = time.perf_counter()
result = fibonacci(30)
elapsed = (time.perf_counter() - start) * 1000
print(f"fibonacci(30) = {result}")
print(f"elapsed_ms={elapsed:.2f}")
