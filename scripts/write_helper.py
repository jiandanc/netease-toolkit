import sys
import pathlib
p = sys.argv[1]
content = sys.stdin.read()
pathlib.Path(p).write_text(content, encoding="utf-8")
print("OK")
