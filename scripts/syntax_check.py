#!/usr/bin/env python3
"""Basic syntax sanity check for the modified core.rs file."""
import re
import sys

path = "/home/z/my-project/app/rs/src/core.rs"
with open(path) as f:
    src = f.read()

# Check brace balance
braces = {"{": 0, "}": 0, "(": 0, ")": 0, "[": 0, "]": 0}
in_string = False
in_char = False
in_line_comment = False
in_block_comment = False
i = 0
while i < len(src):
    c = src[i]
    nxt = src[i+1] if i+1 < len(src) else ""
    # Handle comments
    if in_line_comment:
        if c == "\n":
            in_line_comment = False
        i += 1
        continue
    if in_block_comment:
        if c == "*" and nxt == "/":
            in_block_comment = False
            i += 2
            continue
        i += 1
        continue
    if not in_string and not in_char:
        if c == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue
        if c == "/" and nxt == "*":
            in_block_comment = True
            i += 2
            continue
    # Handle strings
    if c == '"' and not in_char and not in_block_comment:
        # Check for raw string r"..." or r#"..."#
        if i > 0 and src[i-1] == 'r':
            in_string = not in_string
        elif i > 0 and src[i-1] == '#':
            in_string = not in_string
        else:
            in_string = not in_string
    elif c == "'" and not in_string:
        # Could be lifetime or char
        # Simple heuristic: if next char is alphanumeric and char after is ', it's a lifetime
        if i+2 < len(src) and src[i+2] == "'":
            pass  # lifetime, skip
        else:
            in_char = not in_char
    elif not in_string and not in_char:
        if c in braces:
            braces[c] += 1
    i += 1

print("Brace balance (should be 0):")
print(f"  {{ : {braces['{'] - braces['}']}")
print(f"  ( : {braces['('] - braces[')']}")
print(f"  [ : {braces['['] - braces[']']}")


# Check for obvious issues
issues = []
# Check for use of undefined variables
if "init_path" in src:
    # Find all uses of init_path and make sure it's defined before use
    def_idx = src.find("let init_path =")
    use_idx = src.find("cmd.arg(&init_path)")
    if def_idx == -1:
        issues.append("init_path used but never defined with 'let'")
    elif use_idx != -1 and use_idx < def_idx:
        issues.append("init_path used before definition")

# Check for the loader_path variable (should be a function parameter)
if "loader_path" in src and "loader_path: String" not in src:
    issues.append("loader_path used but not declared as parameter")

if issues:
    print("\nISSUES:")
    for i in issues:
        print(f"  - {i}")
    sys.exit(1)
else:
    print("\nNo obvious issues found.")
    print(f"File size: {len(src)} bytes, {src.count(chr(10))} lines")
