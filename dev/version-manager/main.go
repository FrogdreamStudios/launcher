// Project Version Manager.
// This tool helps manage the version number in Cargo.toml file.
// It can increment major, minor, patch versions or add prerelease tags.
package main

import (
	"bufio"
	"fmt"
	"os"
	"regexp"
	"strconv"
	"strings"

	"github.com/Masterminds/semver/v3"
)

// Path to the Cargo.toml file that contains the project version.
const cargoToml = "../../Cargo.toml"

// This function reads the current version from Cargo.toml file.
// Returns a semver.Version object or nil if not found.
func getVersion() *semver.Version {
	file, _ := os.Open(cargoToml)
	defer file.Close()

	scanner := bufio.NewScanner(file)
	inPackageSection := false                                  // Track if we're in the [package] section
	versionRe := regexp.MustCompile(`version\s*=\s*"([^"]+)"`) // Pattern to match version = "1.2.3"

	// Read the file line by line
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())

		// Check if we're entering the [package] section
		if line == "[package]" {
			inPackageSection = true
			continue
		}

		// If we hit another section, we're no longer in [package]
		if strings.HasPrefix(line, "[") && line != "[package]" {
			inPackageSection = false
			continue
		}

		// Look for version line only in the [package] section
		if inPackageSection {
			if matches := versionRe.FindStringSubmatch(line); len(matches) > 1 {
				v, _ := semver.NewVersion(matches[1])
				return v
			}
		}
	}
	return nil
}

// Writes a new version to the Cargo.toml file.
// It finds the version line in the [package] section and replaces it.
func setVersion(v *semver.Version) {
	file, _ := os.Open(cargoToml)
	defer file.Close()

	var lines []string // Store all lines to write back
	scanner := bufio.NewScanner(file)
	inPackageSection := false                                           // Track if we're in [package] section
	versionRe := regexp.MustCompile(`^(\s*version\s*=\s*)"[^"]+"(.*)$`) // Match version line with groups

	// Read all lines and modify the version line
	for scanner.Scan() {
		line := scanner.Text()

		// Check if we're entering the [package] section
		if strings.TrimSpace(line) == "[package]" {
			inPackageSection = true
			lines = append(lines, line)
			continue
		}

		// Check if we're leaving the [package] section
		if strings.HasPrefix(strings.TrimSpace(line), "[") && strings.TrimSpace(line) != "[package]" {
			inPackageSection = false
			lines = append(lines, line)
			continue
		}

		// If we're in [package] section and this is the version line, replace it
		if inPackageSection && versionRe.MatchString(line) {
			matches := versionRe.FindStringSubmatch(line)
			if len(matches) > 2 {

				// Rebuild the line with new version: whitespace + version = "NEW_VERSION" + rest
				newLine := matches[1] + `"` + v.String() + `"` + matches[2]
				lines = append(lines, newLine)
			} else {
				lines = append(lines, line)
			}
		} else {

			// Keep the line as is
			lines = append(lines, line)
		}
	}

	// Write all lines back to the file
	output := strings.Join(lines, "\n")
	_ = os.WriteFile(cargoToml, []byte(output), 0644)
}

// Returns the user's input as a trimmed string.
func prompt(msg string) string {
	fmt.Print(msg)
	scanner := bufio.NewScanner(os.Stdin)
	scanner.Scan()
	return strings.TrimSpace(scanner.Text())
}

// Shows current version and lets user choose how to increment it.
func main() {

	// Get the current version from Cargo.toml
	v := getVersion()

	// Show current version and menu options
	fmt.Printf("Current version: %s\n\n", v)
	fmt.Println("1. Major increment") // 1.2.3 -> 2.0.0
	fmt.Println("2. Minor increment") // 1.2.3 -> 1.3.0
	fmt.Println("3. Patch increment") // 1.2.3 -> 1.2.4
	fmt.Println("4. Prerelease")      // 1.2.3 -> 1.2.4-alpha.1
	fmt.Println("5. Exit")

	choice := prompt("Choice [1-5]: ")
	var newV *semver.Version

	// Handle the user's choice
	switch choice {
	case "1":
		// Major increment: 1.2.3 -> 2.0.0 (big changes)
		var temp = v.IncMajor()
		newV = &temp
	case "2":
		// Minor increment: 1.2.3 -> 1.3.0 (new features)
		temp := v.IncMinor()
		newV = &temp
	case "3":
		// Patch increment: 1.2.3 -> 1.2.4 (bug fixes)
		temp := v.IncPatch()
		newV = &temp
	case "4":
		// Prerelease: 1.2.3 -> 1.2.4-alpha.1 (test versions)
		preType := prompt("Type (alpha/beta/rc): ")
		num := 1 // Default prerelease number
		if numStr := prompt("Number [1]: "); numStr != "" {
			if n, _ := strconv.Atoi(numStr); n > 0 {
				num = n
			}
		}
		// First increment patch, then add prerelease tag
		temp := v.IncPatch()
		newV = &temp
		updated, _ := newV.SetPrerelease(fmt.Sprintf("%s.%d", preType, num))
		newV = &updated
	case "5":
		// Exit without making changes
		return
	default:
		fmt.Println("Invalid choice")
		return
	}

	// Show the new version and ask for confirmation
	fmt.Printf("New version: %s\n", newV)
	if strings.ToLower(prompt("Apply? [y/N]: ")) == "y" {
		setVersion(newV)
		fmt.Printf("Updated to %s\n", newV)
	}
}
