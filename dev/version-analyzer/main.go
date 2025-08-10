// Minecraft Version Analyzer.
// This tool analyzes Minecraft versions and determines what Java version and settings each needs.
package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"github.com/Masterminds/semver/v3"
)

// URL to get the official list of all Minecraft versions from Mojang.
const manifestURL = "https://launchermeta.mojang.com/mc/game/version_manifest.json"

// VersionInfo stores all the information we need about a Minecraft version.
type VersionInfo struct {
	Type        string   // release, snapshot, old_beta, etc.
	JavaVersion int      // which Java version is needed (8, 17, or 21)
	NeedsX86_64 bool     // whether this version requires 64-bit architecture
	JVMFlags    []string // list of Java flags needed to run this version
	ReleaseTime string   // when this version was released
}

// Manifest represents the JSON structure from Mojang's version list.
type Manifest struct {
	Latest   map[string]string `json:"latest"` // latest release and snapshot versions
	Versions []struct {
		ID          string `json:"id"`          // version name like "1.20.1"
		Type        string `json:"type"`        // release, snapshot, etc.
		ReleaseTime string `json:"releaseTime"` // ISO date when released
	} `json:"versions"`
}

// parseVersion extracts version numbers from Minecraft version strings.
// For example, "1.20.1" -> semver object, "23w45a" -> nil.
func parseVersion(v string) *semver.Version {
	// Look for patterns like "1.20" or "1.20.1"
	re := regexp.MustCompile(`\d+\.\d+(\.\d+)?`)
	s := re.FindString(v)
	ver, _ := semver.NewVersion(s)
	return ver
}

// isModernSnapshot checks if a snapshot version is modern enough to need Java 21.
// Modern snapshots are from 2023 onwards (23w prefix) or special experimental versions.
func isModernSnapshot(v string) bool {
	v = strings.ToLower(v)

	// Check for weekly snapshots like "23w45a" - if year >= 23, it's modern
	if m, _ := regexp.MatchString(`^\d{2}w`, v); m && len(v) >= 2 {
		y, _ := strconv.Atoi(v[:2])
		return y >= 23 // 2023 and later snapshots need Java 21
	}

	// Check for pre-release and release candidate versions
	if strings.Contains(v, "-pre") || strings.Contains(v, "-rc") {
		base := strings.Split(v, "-")[0]
		ver := parseVersion(base)

		// If base version is 1.20.5+, it needs Java 21
		return ver != nil && (ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() > 20) || (ver.Major() == 1 && ver.Minor() == 20 && ver.Patch() >= 5))
	}

	// Special experimental versions also need Java 21
	return strings.Contains(v, "experimental") || strings.Contains(v, "snapshot") || strings.Contains(v, "combat")
}

// getJavaVersion determines which Java version a Minecraft version needs.
// Returns 8, 17, or 21 based on the Minecraft version.
func getJavaVersion(v string) int {

	// Modern snapshots always need Java 21
	if isModernSnapshot(v) {
		return 21
	}

	// Very old alpha and beta versions need Java 8
	if strings.HasPrefix(v, "a") || strings.HasPrefix(v, "b") ||
		strings.Contains(v, "alpha") || strings.Contains(v, "beta") {
		return 8
	}

	// Parse the version number to check requirements
	ver := parseVersion(v)
	if ver != nil {

		// Minecraft 1.20.5+ or 1.21+ needs Java 21
		if ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() >= 21) || (ver.Major() == 1 && ver.Minor() == 20 && ver.Patch() >= 5) {
			return 21
		}

		// Minecraft 1.17+ needs Java 17
		if ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() >= 17) {
			return 17
		}
	}

	// Everything else uses Java 8
	return 8
}

// needsX86_64 checks if a Minecraft version requires 64-bit architecture.
// Older versions and some special versions need x86_64.
func needsX86_64(v string) bool {

	// Old alpha and beta versions need 64-bit
	if strings.HasPrefix(v, "a") || strings.HasPrefix(v, "b") ||
		strings.Contains(v, "alpha") || strings.Contains(v, "beta") {
		return true
	}

	ver := parseVersion(v)

	// Versions before 1.18 or unparseable versions need 64-bit
	return ver == nil || ver.Major() < 1 || (ver.Major() == 1 && ver.Minor() < 18)
}

// getJVMFlags creates the list of Java flags needed to run a specific Minecraft version.
// Different Java versions and Minecraft versions need different flags.
func getJVMFlags(javaVer int, mcVer string) []string {

	// Basic flags that all versions need
	f := []string{
		"-Djava.library.path=${natives_directory}",         // Tell Java where to find native libraries
		"-Dminecraft.launcher.brand=${launcher_name}",      // Set launcher name
		"-Dminecraft.launcher.version=${launcher_version}", // Set launcher version
		"-cp ${classpath}", // Set the Java classpath
	}

	// Java 17+ needs special module access flags
	if javaVer >= 17 {
		f = append(f, "--add-opens java.base/java.util.jar=ALL-UNNAMED", "--add-opens java.base/java.lang.invoke=ALL-UNNAMED")
	}

	// Java 21+ needs additional export flags
	if javaVer >= 21 {
		f = append(f, "--add-exports java.base/sun.security.util=ALL-UNNAMED", "--add-exports jdk.naming.dns/com.sun.jndi.dns=java.naming")
	}

	// Set memory and garbage collector based on Minecraft version
	ver := parseVersion(mcVer)
	if ver != nil && (ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() >= 13)) {
		// Minecraft 1.13+ can use more memory and G1 garbage collector
		f = append(f, "-Xmx2G", "-XX:+UseG1GC")
	} else {
		// Older versions use less memory
		f = append(f, "-Xmx1G")
	}

	return f
}

// main function - downloads Minecraft version list and analyzes each version.
func main() {
	fmt.Println("Minecraft version analyzer")

	// Download the official version list from Mojang
	resp, _ := http.Get(manifestURL)
	defer resp.Body.Close()
	body, _ := io.ReadAll(resp.Body)

	// Parse the JSON response
	var manifest Manifest
	_ = json.Unmarshal(body, &manifest)

	// Structure to hold version data for sorting and display
	type entry struct {
		id   string          // version name like "1.20.1"
		info VersionInfo     // analyzed information about this version
		ver  *semver.Version // parsed version for comparison
		time string          // release time for sorting
	}

	// Analyze each version from the manifest
	var versions []entry
	for _, v := range manifest.Versions {
		ver := parseVersion(v.ID)
		versions = append(versions, entry{
			id: v.ID,
			info: VersionInfo{
				Type:        v.Type,
				JavaVersion: getJavaVersion(v.ID),                    // Figure out Java version needed
				NeedsX86_64: needsX86_64(v.ID),                       // Check if 64-bit is required
				JVMFlags:    getJVMFlags(getJavaVersion(v.ID), v.ID), // Generate JVM flags
				ReleaseTime: v.ReleaseTime,
			},
			ver:  ver,
			time: v.ReleaseTime,
		})
	}

	// Sort versions by release time (newest first)
	sort.Slice(versions, func(i, j int) bool {
		return versions[i].time > versions[j].time
	})

	// Print header for the results table
	fmt.Printf("%-20s %-10s %-6s %-8s %-10s %s\n", "Version", "Type", "Java", "x86_64", "Release", "Flags")

	// Print information for each version
	for _, v := range versions {
		// Format the release date (just show year-month-day)
		releaseDate := v.info.ReleaseTime
		if len(releaseDate) >= 10 {
			releaseDate = releaseDate[:10] // Cut off time part, keep just the date
		}

		// Print the main version information
		fmt.Printf("%-20s %-10s %-6d %-8t %-10s %d flags\n",
			v.id, v.info.Type, v.info.JavaVersion, v.info.NeedsX86_64, releaseDate, len(v.info.JVMFlags))

		// Print each JVM flag with indentation
		for _, flag := range v.info.JVMFlags {
			fmt.Printf("    %s\n", flag)
		}
	}
}
