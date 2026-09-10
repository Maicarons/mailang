import time

start = time.perf_counter()
s = 0
for i in range(100000):
    s += i
elapsed = (time.perf_counter() - start) * 1000
print(f"sum = {s}")
print(f"elapsed_ms={elapsed:.2f}")
