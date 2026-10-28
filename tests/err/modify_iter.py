# error: modified while a loop iterates
xs = [1, 2, 3]
for x in xs:
    xs.append(x)
