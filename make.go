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

func main() {
	if len(os.Args) < 2 {
		showHelp()
		return
	}

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

func runCmd(name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func runGoProject(project string) {
	projectPath := filepath.Join(goDevDir, project)
	if _, err := os.Stat(projectPath); os.IsNotExist(err) {
		fmt.Printf("Project not found: %s\n", projectPath)
		os.Exit(1)
	}

	fmt.Printf("Running %s...\n", project)

	cmd := exec.Command("go", "mod", "tidy")
	cmd.Dir = projectPath
	if err := cmd.Run(); err != nil {
		fmt.Printf("Failed to clean modules: %v\n", err)
		os.Exit(1)
	}

	cmd = exec.Command("go", "run", ".")
	cmd.Dir = projectPath
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin

	if err := cmd.Run(); err != nil {
		fmt.Printf("Failed to run %s: %v\n", project, err)
		os.Exit(1)
	}
}

func runGoVersionAnalyzer() {
	runGoProject("version-analyzer")
}

func runGoVersionManager() {
	runGoProject("version-manager")
}

func goBuild() {
	fmt.Println("Building Go projects...")

	projects := []string{"version-analyzer", "version-manager"}
	for _, project := range projects {
		projectPath := filepath.Join(goDevDir, project)
		if _, err := os.Stat(projectPath); os.IsNotExist(err) {
			fmt.Printf("Skipping missing project: %s\n", project)
			continue
		}

		fmt.Printf("Building %s...\n", project)
		cmd := exec.Command("go", "mod", "tidy")
		cmd.Dir = projectPath
		if err := cmd.Run(); err != nil {
			fmt.Printf("Failed to tidy %s: %v\n", project, err)
			continue
		}

		cmd = exec.Command("go", "build", "-o", project, ".")
		cmd.Dir = projectPath
		if err := cmd.Run(); err != nil {
			fmt.Printf("Failed to build %s: %v\n", project, err)
			continue
		}
		fmt.Printf("Built %s\n", project)
	}
}

func goClean() {
	fmt.Println("Cleaning Go artifacts...")

	projects := []string{"version-analyzer", "version-manager"}
	for _, project := range projects {
		projectPath := filepath.Join(goDevDir, project)
		if _, err := os.Stat(projectPath); os.IsNotExist(err) {
			continue
		}

		cmd := exec.Command("go", "clean")
		cmd.Dir = projectPath
		cmd.Run()

		// Remove binary
		binaryPath := filepath.Join(projectPath, project)
		os.Remove(binaryPath)
	}
	fmt.Println("Go clean completed!")
}

func runClippy() {
	fmt.Println("Running Clippy...")
	if err := runCmd("cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"); err != nil {
		fmt.Println("Clippy checks failed")
		os.Exit(1)
	}
	fmt.Println("Clippy passed!")
}

func runFmt() {
	fmt.Println("Formatting code...")
	if err := runCmd("cargo", "fmt"); err != nil {
		fmt.Println("Format failed")
		os.Exit(1)
	}

	fmt.Println("Checking formatting...")
	if err := runCmd("cargo", "fmt", "--", "--check"); err != nil {
		fmt.Println("Format check failed")
		os.Exit(1)
	}
	fmt.Println("Code formatted!")
}

func runBuild() {
	fmt.Println("Building project...")
	if err := runCmd("cargo", "build"); err != nil {
		fmt.Println("Build failed")
		os.Exit(1)
	}
	fmt.Println("Build completed!")
}

func clean() {
	fmt.Println("Cleaning artifacts...")
	goClean()
	if err := runCmd("cargo", "clean"); err != nil {
		fmt.Println("Clean failed")
		os.Exit(1)
	}
	fmt.Println("Clean completed!")
}

func runAll() {
	fmt.Println("Running full build pipeline...")
	runFmt()
	runClippy()
	runBuild()
	fmt.Println("Build pipeline completed!")
}
