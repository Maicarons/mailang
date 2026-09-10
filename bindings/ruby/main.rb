# MaìLang Ruby Binding Demo
# Uses FFI gem to call the C FFI.
#
# Build: gem install ffi && ruby main.rb
# Note: Requires mailang_ffi.dll/so in library path

require 'ffi'

module Mailang
  extend FFI::Library
  ffi_lib 'mailang_ffi'

  attach_function :mailang_create, [], :pointer
  attach_function :mailang_destroy, [:pointer], :void
  attach_function :mailang_free_string, [:pointer], :void
end

puts "=== MaìLang Ruby Binding Demo ===\n"

interp = Mailang.mailang_create
if interp.null?
  STDERR.puts "Failed to create interpreter"
  exit 1
end

puts "Test: Interpreter created successfully"
puts "  Pointer: #{interp}"

Mailang.mailang_destroy(interp)
puts "\n=== Demo Complete ==="
