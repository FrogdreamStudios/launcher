#!/usr/bin/env python3
import json
import re
import requests
from datetime import datetime

MANIFEST_URL = "https://launchermeta.mojang.com/mc/game/version_manifest.json"

def parse_version(version):
    m = re.match(r"(\d+)\.(\d+)(?:\.(\d+))?", version)
    if m:
        major, minor, patch = int(m.group(1)), int(m.group(2)), int(m.group(3) or 0)
        return (major, minor, patch)
    return None

def is_modern_snapshot(version):
    v = version.lower()
    if re.match(r"^\d{2}w", v):
        year = int(v[:2])
        return year >= 23
    if '-pre' in v or '-rc' in v:
        base = v.split('-')[0]
        parsed = parse_version(base)
        return parsed and parsed >= (1, 20, 5)
    if 'experimental' in v or 'snapshot' in v:
        return True
    if 'combat' in v:
        base = v.split('_')[0]
        parsed = parse_version(base)
        return parsed and parsed >= (1, 20, 5)
    return False

def get_java_version(version):
    if is_modern_snapshot(version):
        return 21
    if version.startswith('a') or version.startswith('b') or 'alpha' in version.lower() or 'beta' in version.lower():
        return 8
    parsed = parse_version(version)
    if parsed:
        if parsed >= (1, 21, 0) or parsed >= (1, 20, 5):
            return 21
        if parsed >= (1, 18, 0) or parsed >= (1, 17, 0):
            return 17
        return 8
    return 8

def needs_x86_64(version):
    if version.startswith('a') or version.startswith('b') or 'alpha' in version.lower() or 'beta' in version.lower():
        return True
    parsed = parse_version(version)
    return parsed and parsed < (1, 18, 0)

def get_jvm_flags(java_version, mc_version):
    flags = [
        "-Djava.library.path=${natives_directory}",
        "-Dminecraft.launcher.brand=${launcher_name}",
        "-Dminecraft.launcher.version=${launcher_version}",
        "-cp ${classpath}"
    ]
    if java_version >= 17:
        flags += [
            "--add-opens java.base/java.util.jar=ALL-UNNAMED",
            "--add-opens java.base/java.lang.invoke=ALL-UNNAMED"
        ]
    if java_version >= 21:
        flags += [
            "--add-exports java.base/sun.security.util=ALL-UNNAMED",
            "--add-exports jdk.naming.dns/com.sun.jndi.dns=java.naming"
        ]
    parsed = parse_version(mc_version)
    if parsed and parsed >= (1, 13, 0):
        flags += ["-Xmx2G", "-XX:+UseG1GC"]
    else:
        flags += ["-Xmx1G"]
    return flags

def analyze_versions():
    try:
        resp = requests.get(MANIFEST_URL, timeout=10)
        resp.raise_for_status()
        manifest = resp.json()
    except Exception as e:
        print(f"Error loading manifest: {e}")
        return

    result = {
        "latest": manifest["latest"],
        "analyzed_at": datetime.now().isoformat(),
        "total_versions": len(manifest["versions"]),
        "versions": {}
    }

    for v in manifest["versions"]:
        vid = v["id"]
        java = get_java_version(vid)
        x64 = needs_x86_64(vid)
        flags = get_jvm_flags(java, vid)
        result["versions"][vid] = {
            "type": v["type"],
            "java_version": java,
            "needs_x86_64": x64,
            "jvm_flags": flags,
            "release_time": v["releaseTime"]
        }
    return result

def print_versions(analysis):
    print(f"{'Version':<20} {'Type':<10} {'Java':<6} {'x86_64':<8} {'Release':<10} {'Flags'}")
    for vid, info in sorted(analysis["versions"].items(), key=lambda x: x[1]["release_time"], reverse=True):
        print(f"{vid:<20} {info['type']:<10} {info['java_version']:<6} {str(info['needs_x86_64']):<8} {info['release_time'][:10]:<10} {len(info['jvm_flags'])} flags")
        for flag in info['jvm_flags']:
            print(f"    {flag}")

def save_to_file(analysis, filename="minecraft_versions.json"):
    with open(filename, "w", encoding="utf-8") as f:
        json.dump(analysis, f, indent=2, ensure_ascii=False)
    print(f"Saved to {filename}")

def main():
    print("Minecraft version requirements analyzer")
    analysis = analyze_versions()
    if analysis:
        print_versions(analysis)
        save_to_file(analysis)
    else:
        print("Failed to analyze.")

if __name__ == "__main__":
    main()