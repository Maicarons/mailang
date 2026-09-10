import time

def add(a, b):
    return a + b

start = time.perf_counter()
result = 0
for i in range(100000):
    result = add(result, i)
elapsed = (time.perf_counter() - start) * 1000
print(f"result = {result}")
print(f"elapsed_ms={elapsed:.2f}")
