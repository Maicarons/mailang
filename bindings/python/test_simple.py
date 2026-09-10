"""
MaìLang Python Binding Test - Simple version
"""
import ctypes
import os
import sys

# Load the DLL
dll_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'mailang_ffi.dll')
if not os.path.exists(dll_path):
    print(f"ERROR: DLL not found at {dll_path}")
    sys.exit(1)

print(f"Loading DLL from: {dll_path}")
lib = ctypes.CDLL(dll_path)
print("DLL loaded successfully")

# Define function signatures
lib.mailang_create.restype = ctypes.c_void_p
lib.mailang_destroy.restype = None
lib.mailang_destroy.argtypes = [ctypes.c_void_p]
lib.mailang_free_string.restype = None
lib.mailang_free_string.argtypes = [ctypes.c_char_p]

print("\n=== MaìLang Python Binding Test ===\n")

# Create interpreter
print("Creating interpreter...")
interp = lib.mailang_create()
print(f"Interpreter created: {interp}")

if not interp:
    print("ERROR: Failed to create interpreter")
    sys.exit(1)

# Test basic eval
print("\nTesting eval...")
lib.mailang_eval.restype = ctypes.c_int32
lib.mailang_eval.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

# Simple test - just call eval and check return code
code = b"1 + 2"
print(f"Executing: {code}")
result = lib.mailang_eval(interp, code)
print(f"Result code: {result}")

# Destroy interpreter
print("\nDestroying interpreter...")
lib.mailang_destroy(interp)
print("Done!")

print("\n=== Test Complete ===")
