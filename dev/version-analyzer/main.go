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

const manifestURL = "https://launchermeta.mojang.com/mc/game/version_manifest.json"

type VersionInfo struct {
	Type        string
	JavaVersion int
	NeedsX86_64 bool
	JVMFlags    []string
	ReleaseTime string
}

type Manifest struct {
	Latest   map[string]string `json:"latest"`
	Versions []struct {
		ID          string `json:"id"`
		Type        string `json:"type"`
		ReleaseTime string `json:"releaseTime"`
	} `json:"versions"`
}

func parseVersion(v string) *semver.Version {
	re := regexp.MustCompile(`\d+\.\d+(\.\d+)?`)
	s := re.FindString(v)
	ver, _ := semver.NewVersion(s)
	return ver
}

func isModernSnapshot(v string) bool {
	v = strings.ToLower(v)
	if m, _ := regexp.MatchString(`^\d{2}w`, v); m && len(v) >= 2 {
		y, _ := strconv.Atoi(v[:2])
		return y >= 23
	}
	if strings.Contains(v, "-pre") || strings.Contains(v, "-rc") {
		base := strings.Split(v, "-")[0]
		ver := parseVersion(base)
		return ver != nil && (ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() > 20) || (ver.Major() == 1 && ver.Minor() == 20 && ver.Patch() >= 5))
	}
	return strings.Contains(v, "experimental") || strings.Contains(v, "snapshot") || strings.Contains(v, "combat")
}

func getJavaVersion(v string) int {
	if isModernSnapshot(v) {
		return 21
	}
	if strings.HasPrefix(v, "a") || strings.HasPrefix(v, "b") ||
		strings.Contains(v, "alpha") || strings.Contains(v, "beta") {
		return 8
	}
	ver := parseVersion(v)
	if ver != nil {
		if ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() >= 21) || (ver.Major() == 1 && ver.Minor() == 20 && ver.Patch() >= 5) {
			return 21
		}
		if ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() >= 17) {
			return 17
		}
	}
	return 8
}

func needsX86_64(v string) bool {
	if strings.HasPrefix(v, "a") || strings.HasPrefix(v, "b") ||
		strings.Contains(v, "alpha") || strings.Contains(v, "beta") {
		return true
	}
	ver := parseVersion(v)
	return ver == nil || ver.Major() < 1 || (ver.Major() == 1 && ver.Minor() < 18)
}

func getJVMFlags(javaVer int, mcVer string) []string {
	f := []string{
		"-Djava.library.path=${natives_directory}",
		"-Dminecraft.launcher.brand=${launcher_name}",
		"-Dminecraft.launcher.version=${launcher_version}",
		"-cp ${classpath}",
	}
	if javaVer >= 17 {
		f = append(f, "--add-opens java.base/java.util.jar=ALL-UNNAMED", "--add-opens java.base/java.lang.invoke=ALL-UNNAMED")
	}
	if javaVer >= 21 {
		f = append(f, "--add-exports java.base/sun.security.util=ALL-UNNAMED", "--add-exports jdk.naming.dns/com.sun.jndi.dns=java.naming")
	}
	ver := parseVersion(mcVer)
	if ver != nil && (ver.Major() > 1 || (ver.Major() == 1 && ver.Minor() >= 13)) {
		f = append(f, "-Xmx2G", "-XX:+UseG1GC")
	} else {
		f = append(f, "-Xmx1G")
	}
	return f
}

func main() {
	fmt.Println("Minecraft version analyzer")
	resp, _ := http.Get(manifestURL)
	defer resp.Body.Close()
	body, _ := io.ReadAll(resp.Body)
	var manifest Manifest
	_ = json.Unmarshal(body, &manifest)

	type entry struct {
		id   string
		info VersionInfo
		ver  *semver.Version
		time string
	}
	var versions []entry
	for _, v := range manifest.Versions {
		ver := parseVersion(v.ID)
		versions = append(versions, entry{
			id: v.ID,
			info: VersionInfo{
				Type:        v.Type,
				JavaVersion: getJavaVersion(v.ID),
				NeedsX86_64: needsX86_64(v.ID),
				JVMFlags:    getJVMFlags(getJavaVersion(v.ID), v.ID),
				ReleaseTime: v.ReleaseTime,
			},
			ver:  ver,
			time: v.ReleaseTime,
		})
	}

	sort.Slice(versions, func(i, j int) bool {
		return versions[i].time > versions[j].time
	})

	fmt.Printf("%-20s %-10s %-6s %-8s %-10s %s\n", "Version", "Type", "Java", "x86_64", "Release", "Flags")
	for _, v := range versions {
		releaseDate := v.info.ReleaseTime
		if len(releaseDate) >= 10 {
			releaseDate = releaseDate[:10]
		}
		fmt.Printf("%-20s %-10s %-6d %-8t %-10s %d flags\n",
			v.id, v.info.Type, v.info.JavaVersion, v.info.NeedsX86_64, releaseDate, len(v.info.JVMFlags))
		for _, flag := range v.info.JVMFlags {
			fmt.Printf("    %s\n", flag)
		}
	}
}
