# error: must be a string literal
import re

p = "a+"
print(re.findall(p, "aaa"))
