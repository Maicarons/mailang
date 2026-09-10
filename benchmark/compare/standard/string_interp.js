let start = performance.now();
let result = "";
for (let i = 0; i < 10000; i++) { result = `item_${i}`; }
let elapsed = performance.now() - start;
console.log(`last = ${result}`);
console.log(`elapsed_ms=${elapsed.toFixed(2)}`);
