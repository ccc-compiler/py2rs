# TODO

Known differences from CPython that py2rs does not reject yet (design rule 3: same
output as CPython, or a clear error).

## Set iteration order

`set` is an `IndexSet` (insertion order). CPython iterates in hash-table order, so
`print({3, 1, 2})` shows `{1, 2, 3}` there and `{3, 1, 2}` here; `for x in s`, `list(s)`
and printing differ the same way.

To match it, `pyrt` needs CPython's table (`Objects/setobject.c`):

- `set_add_entry` (9 linear probes, `PERTURB_SHIFT` 5, reuse of the first dummy slot,
  resize when `fill * 5 >= mask * 3` to `used * 4`, or `used * 2` above 50000 entries),
  `set_insert_clean`, `set_table_resize` (minimum size 8), dummies on removal,
  `pop` from the finger
- `set_merge` for copies, `set(s)`, `|`, `|=`; constant literals with 3 or more items
  are a frozenset merged into an empty set (`BUILD_SET 0` + `SET_UPDATE`)
- the algorithms of `&` (iterate the smaller set, the right one when equal), `-`
  (copy-and-discard when `len(a) >> 2 > len(b)`), `^` (a copy of the right operand,
  updated with the left), `-=`, `^=`
- CPython's `hash()`: ints modulo 2**61 - 1 (-1 becomes -2), bools, tuples (xxHash
  based), frozen dataclasses (the tuple of their fields). str hashes are randomized
  per process in CPython, so any order is valid there.

Until then: sort before printing (`sorted(s)`) for output that matches.
