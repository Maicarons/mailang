"""
MaìLang Python Binding Test
Uses ctypes to call the C FFI directly.
"""
import ctypes
import os
import sys

# Load the DLL
dll_path = os.path.join(os.path.dirname(__file__), '..', '..', 'target', 'release', 'mailang_ffi.dll')
if not os.path.exists(dll_path):
    print(f"ERROR: DLL not found at {dll_path}")
    sys.exit(1)

lib = ctypes.CDLL(dll_path)

# Define structures
class MailangResult(ctypes.Structure):
    _fields_ = [
        ("code", ctypes.c_int32),
        ("output", ctypes.c_char_p),
    ]

# Set up function signatures
lib.mailang_create.restype = ctypes.c_void_p
lib.mailang_create.argtypes = []

lib.mailang_destroy.restype = None
lib.mailang_destroy.argtypes = [ctypes.c_void_p]

lib.mailang_eval.restype = MailangResult
lib.mailang_eval.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

lib.mailang_eval_file.restype = MailangResult
lib.mailang_eval_file.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

lib.mailang_free_string.restype = None
lib.mailang_free_string.argtypes = [ctypes.c_char_p]

def test_binding():
    print("=== MaìLang Python Binding Test ===\n")
    
    # Create interpreter
    interp = lib.mailang_create()
    if not interp:
        print("ERROR: Failed to create interpreter")
        return False
    
    tests_passed = 0
    tests_total = 0
    
    def run_test(name, code, expected):
        nonlocal tests_passed, tests_total
        tests_total += 1
        
        result = lib.mailang_eval(interp, code.encode('utf-8'))
        output = result.output.decode('utf-8') if result.output else "null"
        
        if result.output:
            lib.mailang_free_string(result.output)
        
        passed = output == expected
        status = "PASS" if passed else "FAIL"
        
        if passed:
            tests_passed += 1
            print(f"  [{status}] {name}: {code} = {output}")
        else:
            print(f"  [{status}] {name}: {code} = {output} (expected {expected})")
    
    # Test 1: Integer arithmetic
    run_test("Integer Add", "1 + 2", "3")
    run_test("Integer Sub", "10 - 3", "7")
    run_test("Integer Mul", "4 * 5", "20")
    run_test("Integer Div", "10 / 3", "3")
    run_test("Integer Mod", "10 % 3", "1")
    
    # Test 2: Float arithmetic
    run_test("Float Add", "1.5 + 2.5", "4")
    run_test("Float Mul", "3.14 * 2.0", "6.28")
    
    # Test 3: String
    run_test("String Literal", '"hello"', "hello")
    run_test("String Concat", '"Hello, " + "MaìLang!"', "Hello, MaìLang!")
    
    # Test 4: Boolean
    run_test("Boolean True", "true", "true")
    run_test("Boolean False", "false", "false")
    
    # Test 5: Null
    run_test("Null", "null", "null")
    
    # Test 6: Comparison
    run_test("Equal", "1 == 1", "true")
    run_test("Not Equal", "1 != 2", "true")
    run_test("Less Than", "1 < 2", "true")
    run_test("Greater Than", "2 > 1", "true")
    
    # Test 7: Logical
    run_test("Logical And", "true && false", "false")
    run_test("Logical Or", "true || false", "true")
    run_test("Logical Not", "!true", "false")
    
    # Test 8: Variable
    lib.mailang_eval(interp, b"let x = 42")
    result = lib.mailang_eval(interp, b"x")
    output = result.output.decode('utf-8') if result.output else "null"
    if result.output:
        lib.mailang_free_string(result.output)
    tests_total += 1
    if output == "42":
        tests_passed += 1
        print(f"  [PASS] Variable: x = {output}")
    else:
        print(f"  [FAIL] Variable: x = {output} (expected 42)")
    
    # Destroy interpreter
    lib.mailang_destroy(interp)
    
    print(f"\n=== Results: {tests_passed}/{tests_total} tests passed ===")
    return tests_passed == tests_total

if __name__ == "__main__":
    success = test_binding()
    sys.exit(0 if success else 1)
