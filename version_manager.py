#!/usr/bin/env python3

import re

CARGO_TOML = "Cargo.toml"

def get_version():
    with open(CARGO_TOML, "r") as f:
        for line in f:
            m = re.match(r'version\s*=\s*"(.*?)"', line)
            if m:
                return m.group(1)
    print("Version not found in Cargo.toml")
    exit(1)

def set_version(new_version):
    with open(CARGO_TOML, "r") as f:
        lines = f.readlines()
    with open(CARGO_TOML, "w") as f:
        for line in lines:
            if re.match(r'version\s*=', line):
                f.write(f'version = "{new_version}"\n')
            else:
                f.write(line)

def parse_version(version):
    # For example: 1.2.3 or 1.2.3-beta.1
    m = re.match(r"^(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z]+)\.(\d+))?$", version)
    if not m:
        print("Unsupported version format:", version)
        exit(1)
    major, minor, patch = map(int, m.group(1, 2, 3))
    pre = m.group(4)
    pre_num = int(m.group(5)) if m.group(5) else None
    return major, minor, patch, pre, pre_num

def build_version(major, minor, patch, pre=None, pre_num=None):
    v = f"{major}.{minor}.{patch}"
    if pre and pre_num is not None:
        v += f"-{pre}.{pre_num}"
    return v

def select_component(increment=True):
    action = "increment" if increment else "decrement"
    print(f"Select component to {action}:")
    print("  1. Major")
    print("  2. Minor")
    print("  3. Patch")
    print("  4. Pre-release (alpha, beta, rc)")
    choice = input("Enter choice [1-4]: ").strip()
    return choice

def main():
    while True:
        version = get_version()
        print(f"Current version: {version}")
        major, minor, patch, pre, pre_num = parse_version(version)

        print("Select bump type:")
        print("  1. Increment version")
        print("  2. Decrement version")
        print("  3. Restart")
        choice = input("Enter choice [1-3]: ").strip()

        if choice == "3":
            print("Restarting version bump selection")
            continue
        elif choice not in ("1", "2"):
            print("Invalid choice")
            continue

        increment = choice == "1"
        component_choice = select_component(increment)
        if component_choice not in ("1", "2", "3", "4"):
            print("Invalid component choice")
            continue

        inc = input("Enter increment/decrement amount (default 1): ").strip()
        inc = int(inc) if inc.isdigit() else 1
        if inc < 1:
            print("Increment/decrement must be positive")
            continue

        new_major, new_minor, new_patch = major, minor, patch
        new_pre, new_pre_num = pre, pre_num

        if component_choice == "1":  # Major
            if increment:
                new_major += inc
                new_minor = 0
                new_patch = 0
                new_pre = None
                new_pre_num = None
            else:
                new_major = max(0, major - inc)
                if new_major == major:
                    print("Cannot decrement major version below 0")
                    continue
                new_minor = 0
                new_patch = 0
                new_pre = None
                new_pre_num = None
        elif component_choice == "2":  # Minor
            if increment:
                new_minor += inc
                new_patch = 0
                new_pre = None
                new_pre_num = None
            else:
                new_minor = max(0, minor - inc)
                if new_minor == minor:
                    print("Cannot decrement minor version below 0")
                    continue
                new_patch = 0
                new_pre = None
                new_pre_num = None
        elif component_choice == "3":  # Patch
            if increment:
                new_patch += inc
                new_pre = None
                new_pre_num = None
            else:
                new_patch = max(0, patch - inc)
                if new_patch == patch:
                    print("Cannot decrement patch version below 0")
                    continue
                new_pre = None
                new_pre_num = None
        elif component_choice == "4":  # Prerelease
            if increment:
                print("Select pre-release type:")
                print("  1. alpha")
                print("  2. beta")
                print("  3. rc")
                pre_choice = input("Enter choice [1-3]: ").strip()
                pre_types = {"1": "alpha", "2": "beta", "3": "rc"}
                if pre_choice not in pre_types:
                    print("Invalid prerelease type")
                    continue
                new_pre_type = pre_types[pre_choice]
                if pre == new_pre_type:
                    new_pre_num = (pre_num or 0) + inc
                else:
                    new_pre = new_pre_type
                    new_pre_num = 1
            else:
                if not pre or pre_num is None:
                    print("No prerelease version to decrement")
                    continue
                new_pre_num = max(0, pre_num - inc)
                if new_pre_num == 0:
                    new_pre = None
                    new_pre_num = None

        new_version = build_version(new_major, new_minor, new_patch, new_pre, new_pre_num)
        print(f"New version will be: {new_version}")
        confirm = input("Apply? [y/N]: ").strip().lower()
        if confirm == "y":
            set_version(new_version)
            print(f"Version updated to {new_version}")
        else:
            print("Aborted")
        break

if __name__ == "__main__":
    main()
