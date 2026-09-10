function add(a, b) { return a + b; }
let start = performance.now();
let result = 0;
for (let i = 0; i < 100000; i++) { result = add(result, i); }
let elapsed = performance.now() - start;
console.log(`result = ${result}`);
console.log(`elapsed_ms=${elapsed.toFixed(2)}`);
