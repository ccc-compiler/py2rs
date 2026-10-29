# regression: Wrong output: round(int, n) must return an int, py2rs returns a float.
print(round(1250, -2), round(1350, -2), round(-15, -1), round(7, 2))
