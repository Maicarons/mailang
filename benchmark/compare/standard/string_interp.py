import time

start = time.perf_counter()
result = ""
for i in range(10000):
    result = f"item_{i}"
elapsed = (time.perf_counter() - start) * 1000
print(f"last = {result}")
print(f"elapsed_ms={elapsed:.2f}")
