# Python demo for MaìLang
# Requires: pip install mailang

from mailang import MailangInterpreter

interp = MailangInterpreter()

result = interp.eval('1 + 2')
print(f"Result: {result}")

result = interp.eval('"Hello, " + "MaìLang!"')
print(f"Result: {result}")
