let start = performance.now();
let s = 0;
for (let i = 0; i < 100000; i++) { s += i; }
let elapsed = performance.now() - start;
console.log(`sum = ${s}`);
console.log(`elapsed_ms=${elapsed.toFixed(2)}`);
