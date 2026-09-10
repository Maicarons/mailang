// Node.js demo for MaìLang
// Requires: npm install mailang

const { MailangInterpreter } = require('mailang');

const interp = new MailangInterpreter();

console.log(interp.eval('1 + 2'));
console.log(interp.eval('"Hello, " + "MaìLang!"'));
