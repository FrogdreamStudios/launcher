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

const cargoToml = "../../Cargo.toml"

func getVersion() *semver.Version {
	file, _ := os.Open(cargoToml)
	defer file.Close()

	scanner := bufio.NewScanner(file)
	inPackageSection := false
	versionRe := regexp.MustCompile(`version\s*=\s*"([^"]+)"`)

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "[package]" {
			inPackageSection = true
			continue
		}
		if strings.HasPrefix(line, "[") && line != "[package]" {
			inPackageSection = false
			continue
		}
		if inPackageSection {
			if matches := versionRe.FindStringSubmatch(line); len(matches) > 1 {
				v, _ := semver.NewVersion(matches[1])
				return v
			}
		}
	}
	return nil
}

func setVersion(v *semver.Version) {
	file, _ := os.Open(cargoToml)
	defer file.Close()

	var lines []string
	scanner := bufio.NewScanner(file)
	inPackageSection := false
	versionRe := regexp.MustCompile(`^(\s*version\s*=\s*)"[^"]+"(.*)$`)

	for scanner.Scan() {
		line := scanner.Text()
		if strings.TrimSpace(line) == "[package]" {
			inPackageSection = true
			lines = append(lines, line)
			continue
		}
		if strings.HasPrefix(strings.TrimSpace(line), "[") && strings.TrimSpace(line) != "[package]" {
			inPackageSection = false
			lines = append(lines, line)
			continue
		}
		if inPackageSection && versionRe.MatchString(line) {
			matches := versionRe.FindStringSubmatch(line)
			if len(matches) > 2 {
				newLine := matches[1] + `"` + v.String() + `"` + matches[2]
				lines = append(lines, newLine)
			} else {
				lines = append(lines, line)
			}
		} else {
			lines = append(lines, line)
		}
	}

	output := strings.Join(lines, "\n")
	_ = os.WriteFile(cargoToml, []byte(output), 0644)
}

func prompt(msg string) string {
	fmt.Print(msg)
	scanner := bufio.NewScanner(os.Stdin)
	scanner.Scan()
	return strings.TrimSpace(scanner.Text())
}

func main() {
	v := getVersion()

	fmt.Printf("Current version: %s\n\n", v)
	fmt.Println("1. Major increment")
	fmt.Println("2. Minor increment")
	fmt.Println("3. Patch increment")
	fmt.Println("4. Prerelease")
	fmt.Println("5. Exit")

	choice := prompt("Choice [1-5]: ")
	var newV *semver.Version

	switch choice {
	case "1":
		var temp = v.IncMajor()
		newV = &temp
	case "2":
		temp := v.IncMinor()
		newV = &temp
	case "3":
		temp := v.IncPatch()
		newV = &temp
	case "4":
		preType := prompt("Type (alpha/beta/rc): ")
		num := 1
		if numStr := prompt("Number [1]: "); numStr != "" {
			if n, _ := strconv.Atoi(numStr); n > 0 {
				num = n
			}
		}
		temp := v.IncPatch()
		newV = &temp
		updated, _ := newV.SetPrerelease(fmt.Sprintf("%s.%d", preType, num))
		newV = &updated
	case "5":
		return
	default:
		fmt.Println("Invalid choice")
		return
	}

	fmt.Printf("New version: %s\n", newV)
	if strings.ToLower(prompt("Apply? [y/N]: ")) == "y" {
		setVersion(newV)
		fmt.Printf("Updated to %s\n", newV)
	}
}
