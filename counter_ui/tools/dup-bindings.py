#!/usr/bin/env python3
"""Find a property bound twice inside one QML object block.

qmllint passes such a file and the host then dies at load, which in a Basecamp
view means a silent dead panel (playbook gotcha 30). This walks brace depth and
flags a `name:` assigned twice at the same depth within the same block.
"""
import re, sys, pathlib

ASSIGN = re.compile(r'^\s*((?:[A-Za-z_][\w.]*\.)?[A-Za-z_]\w*)\s*:(?!:)')
SKIP = {"function", "property", "readonly", "signal", "required", "default",
        "case", "default:", "import", "pragma", "return"}

def scan(path):
    bad = []
    stack = [{}]          # one dict of seen names per open block
    for n, line in enumerate(path.read_text().splitlines(), 1):
        code = re.sub(r'//.*$', '', line)
        m = ASSIGN.match(code)
        if m:
            name = m.group(1)
            head = code.strip().split()[0].rstrip(':')
            if head not in SKIP and name not in SKIP:
                seen = stack[-1]
                if name in seen:
                    bad.append((n, name, seen[name]))
                else:
                    seen[name] = n
        for ch in code:
            if ch == '{':
                stack.append({})
            elif ch == '}' and len(stack) > 1:
                stack.pop()
    return bad

rc = 0
for arg in sys.argv[1:]:
    for f in sorted(pathlib.Path(arg).glob("*.qml")) if pathlib.Path(arg).is_dir() else [pathlib.Path(arg)]:
        for n, name, first in scan(f):
            print(f"{f}:{n}: '{name}' already bound at line {first}")
            rc = 1
print("no duplicate bindings" if rc == 0 else "", end="")
sys.exit(rc)
