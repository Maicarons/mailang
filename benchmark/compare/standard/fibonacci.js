// QuickJS-style benchmark (no performance.now, use Date)
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

_ = fibonacci(10);
var start = new Date();
var result = fibonacci(30);
var elapsed = new Date() - start;
print("fibonacci(30) = " + result);
print("elapsed_ms=" + elapsed);
