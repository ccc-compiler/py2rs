# error: lookahead / lookbehind
import re

print(re.findall(r"\d+(?=px)", "10px 20em"))
