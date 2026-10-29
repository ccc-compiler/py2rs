# py2rs

[English](README.md) · [繁體中文](README.tw.md) · 简体中文

把有类型注解的 Python 子集翻译成可读的 Rust。

```python
@dataclass
class Item:
    name: str
    price: float

def total(items: list[Item], tax: float = 0.05) -> float:
    return sum(i.price for i in items) * (1 + tax)
```

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    pub price: f64,
}

fn total(items: &[Item], tax: f64) -> f64 {
    pyrt::sum_f64(items.iter().map(|i| i.price)) * (1.0 + tax)
}
```

## 保证

1. **每个值都有静态类型。** 函数要注解类型，局部变量由推导得出。
2. **看不出共享的差别。** Python 的名称会共享可变对象，Rust 则是拷贝、移动或借用。
   只有在差别看不出来时 py2rs 才接受程序，并为每个参数推导出 `&T` / `&mut T` / `T`。
   （`@shared` 类保留 Python 的共享语义，以 `Rc<RefCell<..>>` 实现。）
3. **输出和 CPython 一样，否则清楚报错。** 无法保证的写法会在翻译时被拒绝，附上行号和
   提示，绝不会翻成行为不同的程序。
4. **可以直接使用 Rust crate**（`from rust.regex import Regex`）。

## 安装与使用

```sh
cargo install py2rs            # 或在原代码目录运行 cargo build --release
py2rs emit  prog.py            # 印出 Rust 程序
py2rs check prog.py            # 只检查类型与所有权
py2rs explain prog.py          # 参数模式、引用、拷贝、返回 Result 的函数
py2rs build prog.py [-o DIR]   # 产生 Cargo 项目并以 release 构建
py2rs run   prog.py            # 构建（有缓存）并运行
py2rs pyo3  mod.py [-o DIR]    # Python 扩展模块（PyO3）：DIR/mod.so
py2rs test                     # 与 CPython 的差分测试（在原代码目录）
py2rs fuzz 300                 # 随机程序与 CPython 比对
```

程序可以 import 自己的模块（`import geometry`、`from geometry import Point`，
`geometry.py` 放在同一个目录）。

## 支持范围

完整定义在 [SPEC.md](SPEC.md)（英文）。简单说：`int`（64 位，溢出会报错）、
`float`、`bool`、`str`、`list`、`dict`、`set`、`tuple`、`Optional`；有默认值、关键字
参数与 `*args` 的函数；dataclass 与一般类（方法、property、classmethod、操作符方法、
单一继承）；异常（也可以有字段）与 `try` / `finally`；推导式、f-string、`match`、
`with open`、generator、使用 `nonlocal` 的嵌套函数、lambda 与 `Callable`；泛型函数、
`Protocol` 与联合类型参数（依参数类型分别编译）；`math`、`collections`、`heapq`、
`bisect`、`itertools`、`functools`、`random`（同一个 seed 产生和 CPython 一样的数字）、
`re`、`sys`、`time`、`json.dumps`、`os.path`。

一些写法翻译后的样子（更多在 [docs/cases](docs/cases/README.md)）：

| Python | Rust |
|---|---|
| `x: T \| None`、`if x is not None:` | `Option<T>`、`if let Some(x) = &x` |
| `node.next = self.head; self.head = node` | `node.next = self.head.take();` |
| 只读取的局部变量（`cur = cur.next`） | 引用：`let mut cur = &self.head;` |
| `item = self.items[k]; item.qty -= 1` | `let item = &mut self.items[k];` |
| 有子类的类 | 枚举已创建的类的 `enum`，方法用 `match` 分派 |
| 调用者会接住的 `raise` | 只在那条路径上返回 `Result<T, PyErr>`，使用 `?` |
| 使用 `seen` 和 `nonlocal count` 的嵌套函数 | `fn dfs(.., seen: &mut HashSet<..>, count: &mut i64)`，每次调用时传入 |
| `def first[T](xs: list[T]) -> T` | `first_int(xs: &[i64])`、`first_str(xs: &[String])`，每种参数类型一份 |
| `def gen(n) -> Iterator[int]: ... yield i` | `pyrt::Gen<i64>`：函数本体和使用端严格轮流运行 |
| `@shared class Node` | `struct Node(Rc<RefCell<NodeData>>)`，加上 getter 和 setter |

与 CPython 已知的差异：`int` 是 64 位（超出范围会引发 `OverflowError`）；set 的遍历
顺序是插入顺序（见 [TODO.md](TODO.md)）。

## 速度

`python3 bench/run.py`，CPython 3.14 与翻译后的程序比较（Apple silicon）：

| 程序 | CPython | Rust | |
|---|---:|---:|---:|
| nbody（浮点数对象） | 1.21 s | 0.012 s | 101 倍 |
| matmul（浮点数列表） | 0.47 s | 0.013 s | 38 倍 |
| fib（递归） | 0.26 s | 0.013 s | 20 倍 |
| sieve | 0.48 s | 0.027 s | 18 倍 |
| graph（用 deque 的 BFS） | 0.14 s | 0.017 s | 8 倍 |
| objects（dataclass、排序、dict） | 0.59 s | 0.10 s | 6 倍 |
| printing | 0.21 s | 0.056 s | 4 倍 |
| strings、wordfreq | | | 2 倍 |

## 所有权注解

参数模式是推导出来的。要固定某个参数的模式就加上注解（`python/rust` 模块让这些注解在
CPython 下也能运行；把该目录加进 `PYTHONPATH`）：

```python
from rust import Ref, Mut, Own

def fill(xs: Mut[list[int]], n: int) -> None: ...   # &mut Vec<i64>
def keep(xs: Own[list[int]]) -> None: ...           # Vec<i64>
```

代码和注解矛盾时（例如修改 `Ref[...]`）会报错。

## Rust crate

`from rust.CRATE import Name` 使用 Rust crate。声明来自 stub 档 `CRATE.pyi`，依序在
`./stubs`、`PY2RS_STUBS` 和 py2rs 附带的 stub（`stubs/regex.pyi`）中寻找；该 crate
会成为产生出来的项目的 Cargo 依赖。

```python
from rust.regex import Regex

date = Regex.new(r"(\d{4})-(\d{2})-(\d{2})")      # Result → 无效时引发 ValueError
print(date.replace_all("on 2024-05-17", "$3/$2/$1"))
```

stub 是有类型声明的 Python 档，每个声明对应一个 Rust 路径或表达式范本
（`@rust("{self}.find({0}).map(|m| m.as_str().to_string())")`）。`py2rs stub
CRATE[@VERSION]` 会从 crate 的 rustdoc JSON 产生 stub。`from rust import inline` 是
后门：`inline("xs.sort_unstable();")`、`inline[int]("n.count_ones() as i64")`。

## Python 扩展模块

`py2rs pyo3 mod.py` 构建出 `mod.so`，Python 可以像原本的模块一样 import。模块层级的
函数会被导出（参数与返回值可以是 int、float、bool、str、list、tuple、dict 和
`None`）；函数引发的异常会在 Python 中以相同类引发；`print` 写到 Python 的
`sys.stdout`。`tests/pyo3/run.sh` 检查 import 这个模块印出的内容和 import 原本的
Python 档相同。

## 测试

`py2rs test [文件|目录]`（缺省 `tests/`）翻译每个程序，全部放进一个 Cargo 包构建，
再和 CPython（`PY2RS_PYTHON`，否则 python3.14 / 3.13 / 3.12）比较标准输出、结束码和
异常类型。有 `NAME.in` 时当作输入。第一行是 `# error: TEXT` 的程序必须被拒绝，而且
错误消息要包含 TEXT（`tests/err/`）。测试集约有 240 个程序和 45 个拒绝案例；
`py2rs fuzz` 另外产生随机程序。

## 原代码结构

- `src/front.rs`（及 `src/front/`）：ruff 解析的 AST → 有类型的 IR（`src/ir.rs`）；
  标准库、`re`、操作符方法、嵌套函数、`match`、泛型
- `src/own.rs`：所有权推导、活跃性分析、共享与遍历检查、引用
- `src/exc.rs`：哪些异常会在哪里被接住，因而决定哪些函数返回 `Result`
- `src/gen.rs`（及 `src/gen/`）：IR → Rust
- `src/modules.rs`：多文件程序；`src/pyo3.rs`：扩展模块
- `pyrt/`：运行时库：Python 的输出格式、格式规格、`//` `%` `**` 语义、以字符为
  单位的 `str` 方法、异常、标准库、generator、缓冲输出
