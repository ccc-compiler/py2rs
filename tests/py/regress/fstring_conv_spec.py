# regression: Rejected: "format spec `>6` does not fit this value" for a valid f-string.
b = False
s = "a"
print(f"[{b!s:>6}] [{s!r:<6}]")
