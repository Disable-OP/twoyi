#!/usr/bin/env python3
"""Patch twoyi_api.cpp to remove the dl*_ex stubs (now in dl_ex.cpp)."""
import re
import sys

path = sys.argv[1]
with open(path) as f:
    src = f.read()

# Remove the four dl*_ex function definitions. They're between the
# "// dlopen_ex / dlsym_ex / dlclose_ex / dlerror_ex: thin wrappers..." comment
# and the closing "} // extern \"C\"".
pattern = re.compile(
    r"// dlopen_ex / dlsym_ex / dlclose_ex / dlerror_ex: thin wrappers.*?const char\* dlerror_ex\(void\)\n\{\n    return dlerror\(\);\n\}\n\n",
    re.DOTALL,
)
new_src, n = pattern.subn("", src)
if n != 1:
    print(f"ERROR: expected 1 replacement, got {n}", file=sys.stderr)
    sys.exit(1)

with open(path, "w") as f:
    f.write(new_src)
print(f"Patched {path}: removed {n} dl*_ex stub block(s).")
