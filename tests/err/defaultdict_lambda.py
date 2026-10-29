# error: needs a factory
from collections import defaultdict

d = defaultdict(lambda: 5)
print(d["x"])
