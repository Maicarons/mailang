// MaìLang Go Binding Test
// This test verifies the C FFI binding works from Go.
//
// Build: go build -o test_binding.exe test_binding.go
// Note: Requires the DLL to be in the same directory or PATH.

package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

func main() {
	fmt.Println("=== MaìLang Go Binding Test ===")
	fmt.Println()

	// Find the C demo executable
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)
	demoPath := filepath.Join(dir, "..", "c", "demo.exe")

	if _, err := os.Stat(demoPath); os.IsNotExist(err) {
		fmt.Println("C demo not found. Building it first...")
		
		// Try to build the C demo
		cDir := filepath.Join(dir, "..", "c")
		cmd := exec.Command("gcc", "-o", "demo.exe", "main.c", "-I.", 
			"-L../../../target/x86_64-pc-windows-gnu/release",
			"-lmailang_ffi", "-lws2_32", "-luserenv", "-lntdll")
		cmd.Dir = cDir
		output, err := cmd.CombinedOutput()
		if err != nil {
			fmt.Printf("Failed to build C demo: %s\n%s\n", err, output)
			os.Exit(1)
		}
	}

	// Run the C demo
	fmt.Println("Running C demo via Go...")
	cmd := exec.Command(demoPath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		fmt.Printf("Failed to run C demo: %s\n", err)
		os.Exit(1)
	}

	fmt.Println(string(output))
	fmt.Println("=== Go Binding Test Complete ===")
}
