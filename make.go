// Build tool for managing the launcher project.
// This tool helps coders to build, formate and do some other stuff.
package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

const (
	goDevDir = "dev"
)

// This function handles command line arguments and runs the right command.
func main() {
	if len(os.Args) < 2 {
		showHelp()
		return
	}

	// Run the right command based on the first argument
	switch os.Args[1] {
	case "help", "h", "--help":
		showHelp()
	case "go-version-analyzer":
		runGoVersionAnalyzer()
	case "go-version-manager":
		runGoVersionManager()
	case "go-build":
		goBuild()
	case "go-clean":
		goClean()
	case "clippy":
		runClippy()
	case "fmt":
		runFmt()
	case "clean":
		clean()
	case "all":
		runAll()
	default:
		fmt.Printf("Unknown command: %s\n", os.Args[1])
		showHelp()
		os.Exit(1)
	}
}

// Shows the help message with all available commands.
func showHelp() {
	fmt.Println("Developer Tools")
	fmt.Println("\nAvailable commands:")
	fmt.Println("  help                 – Show this help")
	fmt.Println("  all                  – Run fmt, Clippy, and build")
	fmt.Println("  fmt                  – Format code")
	fmt.Println("  clippy               – Run Clippy linter")
	fmt.Println("  clean                – Clean build artifacts")
	fmt.Println("  go-version-analyzer  – Run Go version analyzer")
	fmt.Println("  go-version-manager   – Run Go version manager")
	fmt.Println("  go-build             – Run Go projects")
	fmt.Println("  go-clean             – Run Go clean artifacts")
}

// Helper function to run a command and show its output.
func runCmd(name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Stdout = os.Stdout // Show command output to user
	cmd.Stderr = os.Stderr // Show command errors to user
	return cmd.Run()
}

// Runs a specific Go project from the dev directory.
func runGoProject(project string) {
	// Build the full path to the project
	projectPath := filepath.Join(goDevDir, project)

	// Check if the project directory exists
	if _, err := os.Stat(projectPath); os.IsNotExist(err) {
		fmt.Printf("Project not found: %s\n", projectPath)
		os.Exit(1)
	}

	fmt.Printf("Running %s...\n", project)

	// First, clean up Go modules because it might have outdated dependencies
	cmd := exec.Command("go", "mod", "tidy")
	cmd.Dir = projectPath
	if err := cmd.Run(); err != nil {
		fmt.Printf("Failed to clean modules: %v\n", err)
		os.Exit(1)
	}

	// Run the Go project
	cmd = exec.Command("go", "run", ".")
	cmd.Dir = projectPath
	cmd.Stdout = os.Stdout // Show output to user
	cmd.Stderr = os.Stderr // Show errors to user
	cmd.Stdin = os.Stdin   // Allow user input

	if err := cmd.Run(); err != nil {
		fmt.Printf("Failed to run %s: %v\n", project, err)
		os.Exit(1)
	}
}

// Runs the Minecraft version analyzer tool.
// You can find more information about this (and other tools) in the dev directory.
func runGoVersionAnalyzer() {
	runGoProject("version-analyzer")
}

// Runs the project version manager tool
func runGoVersionManager() {
	runGoProject("version-manager")
}

// Builds all Go projects and creates executable files
func goBuild() {
	fmt.Println("Building Go projects...")

	// List of all Go projects to build
	projects := []string{"version-analyzer", "version-manager"}

	for _, project := range projects {
		projectPath := filepath.Join(goDevDir, project)

		// Skip if project doesn't exist
		if _, err := os.Stat(projectPath); os.IsNotExist(err) {
			fmt.Printf("Skipping missing project: %s\n", project)
			continue
		}

		// Check if the project has a go.mod file
		fmt.Printf("Building %s...\n", project)

		// Clean up dependencies first
		cmd := exec.Command("go", "mod", "tidy")
		cmd.Dir = projectPath
		if err := cmd.Run(); err != nil {
			fmt.Printf("Failed to tidy %s: %v\n", project, err)
			continue
		}

		// Build the project into an executable file
		cmd = exec.Command("go", "build", "-o", project, ".")
		cmd.Dir = projectPath
		if err := cmd.Run(); err != nil {
			fmt.Printf("Failed to build %s: %v\n", project, err)
			continue
		}
		fmt.Printf("Built %s\n", project)
	}
}

// Cleans up Go build files and removes executable files.
func goClean() {
	fmt.Println("Cleaning Go artifacts...")

	// List of projects to clean
	projects := []string{"version-analyzer", "version-manager"}

	for _, project := range projects {
		projectPath := filepath.Join(goDevDir, project)

		// Skip if project doesn't exist
		if _, err := os.Stat(projectPath); os.IsNotExist(err) {
			continue
		}

		// Run go clean to remove the build cache
		cmd := exec.Command("go", "clean")
		cmd.Dir = projectPath
		cmd.Run()

		// Remove the executable file we built
		binaryPath := filepath.Join(projectPath, project)
		os.Remove(binaryPath)
	}
	fmt.Println("Go clean completed!")
}

// Runs Clippy linter to check Rust code for problems.
// Please note that warnings are treated as errors. This helps us to keep our codebase clean.
// Run this command before committing any changes.
// It will help you find problems in your changes.
func runClippy() {
	fmt.Println("Running Clippy...")
	if err := runCmd("cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"); err != nil {
		fmt.Println("Clippy checks failed")
		os.Exit(1)
	}
	fmt.Println("Clippy passed!")
}

// Formats Rust code automatically and checks if it's properly formatted.
// Run this command before committing any changes like with Clippy.
// It will help you keep your code clean and consistent.
func runFmt() {
	fmt.Println("Formatting code...")
	if err := runCmd("cargo", "fmt"); err != nil {
		fmt.Println("Format failed")
		os.Exit(1)
	}

	fmt.Println("Checking formatting...")
	// Check if code is properly formatted
	if err := runCmd("cargo", "fmt", "--", "--check"); err != nil {
		fmt.Println("Format check failed")
		os.Exit(1)
	}
	fmt.Println("Code formatted!")
}

// / Builds the main Rust project.
func runBuild() {
	fmt.Println("Building project...")
	// Build the Rust project using Cargo
	if err := runCmd("cargo", "build"); err != nil {
		fmt.Println("Build failed")
		os.Exit(1)
	}
	fmt.Println("Build completed!")
}

// / Cleans all build artifacts from both Go and Rust projects.
func clean() {
	fmt.Println("Cleaning artifacts...")

	// Clean Go projects first
	goClean()

	// Then clean Rust project
	if err := runCmd("cargo", "clean"); err != nil {
		fmt.Println("Clean failed")
		os.Exit(1)
	}
	fmt.Println("Clean completed!")
}

// / Runs the complete build pipeline: format, lint, and build.
func runAll() {
	fmt.Println("Running full build pipeline...")
	runFmt()    // Format code first
	runClippy() // Check for problems
	runBuild()  // Build the project
	fmt.Println("Build pipeline completed!")
}
