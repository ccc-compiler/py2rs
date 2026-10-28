# error: still reachable
path = [1]
result: list[list[int]] = []
result.append(path)
path.append(2)
print(result)
