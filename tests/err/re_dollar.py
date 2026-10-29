# error: also matches before a final newline
import re

print(re.search(r"end$", "the end\n") is not None)
