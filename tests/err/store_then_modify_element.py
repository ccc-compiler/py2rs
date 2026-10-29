# error: shares with the container
rows: list[list[int]] = []
row = [1]
rows.append(row)
rows[0].append(2)
print(rows, row)
