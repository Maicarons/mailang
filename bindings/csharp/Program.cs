/**
 * MaìLang C# Binding Demo
 * Uses P/Invoke to call the C FFI directly.
 *
 * Build: dotnet run
 * Note: Requires mailang_ffi.dll in the output directory
 */
using System;
using System.Runtime.InteropServices;

class Program
{
    [StructLayout(LayoutKind.Sequential)]
    struct MailangResult
    {
        public int Code;
        public IntPtr Output;
    }

    [DllImport("mailang_ffi", CallingConvention = CallingConvention.Cdecl)]
    static extern IntPtr mailang_create();

    [DllImport("mailang_ffi", CallingConvention = CallingConvention.Cdecl)]
    static extern void mailang_destroy(IntPtr interp);

    [DllImport("mailang_ffi", CallingConvention = CallingConvention.Cdecl)]
    static extern int mailang_eval(IntPtr interp, string code, out MailangResult result);

    [DllImport("mailang_ffi", CallingConvention = CallingConvention.Cdecl)]
    static extern void mailang_free_string(IntPtr ptr);

    static void Main(string[] args)
    {
        Console.WriteLine("=== MaìLang C# Binding Demo ===\n");

        IntPtr interp = mailang_create();
        if (interp == IntPtr.Zero)
        {
            Console.Error.WriteLine("Failed to create interpreter");
            return;
        }

        // Test basic arithmetic
        Console.WriteLine("Test: Integer arithmetic");
        var ret = mailang_eval(interp, "1 + 2", out var result);
        if (ret == 0 && result.Code == 0)
        {
            string output = Marshal.PtrToStringAnsi(result.Output) ?? "null";
            Console.WriteLine($"  1 + 2 = {output}");
            mailang_free_string(result.Output);
        }

        // Test string
        Console.WriteLine("Test: String concatenation");
        ret = mailang_eval(interp, "\"Hello, \" + \"MaìLang!\"", out result);
        if (ret == 0 && result.Code == 0)
        {
            string output = Marshal.PtrToStringAnsi(result.Output) ?? "null";
            Console.WriteLine($"  Result: {output}");
            mailang_free_string(result.Output);
        }

        mailang_destroy(interp);
        Console.WriteLine("\n=== Demo Complete ===");
    }
}
