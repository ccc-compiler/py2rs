# regression: Rust compile error E0689: can't call method `abs` on ambiguous numeric type `{float}`.
print(abs(0.1 + 0.2 - 0.3) < 1e-9, abs(0.1 + 0.2 - 0.3))
