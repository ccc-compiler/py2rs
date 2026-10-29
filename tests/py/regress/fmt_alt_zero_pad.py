# regression: Wrong output: '#' (alternate form) with 0-padding puts the zeros before the 0x/0b prefix.
print(f"{255:#06x} {5:#06b} {-255:#07x}")
