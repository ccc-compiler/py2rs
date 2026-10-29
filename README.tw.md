# py2rs

[English](README.md) · 繁體中文 · [简体中文](README.cn.md)

把有型別標註的 Python 子集翻譯成可讀的 Rust。

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

## 保證

1. **每個值都有靜態型別。** 函式要標註型別，區域變數由推導得出。
2. **看不出共享的差別。** Python 的名稱會共享可變物件，Rust 則是複製、移動或借用。
   只有在差別看不出來時 py2rs 才接受程式，並為每個參數推導出 `&T` / `&mut T` / `T`。
   （`@shared` 類別保留 Python 的共享語意，以 `Rc<RefCell<..>>` 實作。）
3. **輸出和 CPython 一樣，否則清楚報錯。** 無法保證的寫法會在翻譯時被拒絕，附上行號和
   提示，絕不會翻成行為不同的程式。
4. **可以直接使用 Rust crate**（`from rust.regex import Regex`）。

## 安裝與使用

```sh
cargo install py2rs            # 或在原始碼目錄執行 cargo build --release
py2rs emit  prog.py            # 印出 Rust 程式
py2rs check prog.py            # 只檢查型別與所有權
py2rs explain prog.py          # 參數模式、參考、複製、回傳 Result 的函式
py2rs build prog.py [-o DIR]   # 產生 Cargo 專案並以 release 建置
py2rs run   prog.py            # 建置（有快取）並執行
py2rs pyo3  mod.py [-o DIR]    # Python 擴充模組（PyO3）：DIR/mod.so
py2rs test                     # 與 CPython 的差分測試（在原始碼目錄）
py2rs fuzz 300                 # 隨機程式與 CPython 比對
```

程式可以 import 自己的模組（`import geometry`、`from geometry import Point`，
`geometry.py` 放在同一個目錄）。

## 支援範圍

完整定義在 [SPEC.md](SPEC.md)（英文）。簡單說：`int`（64 位元，溢位會報錯）、
`float`、`bool`、`str`、`list`、`dict`、`set`、`tuple`、`Optional`；有預設值、關鍵字
參數與 `*args` 的函式；dataclass 與一般類別（方法、property、classmethod、運算子方法、
單一繼承）；例外（也可以有欄位）與 `try` / `finally`；推導式、f-string、`match`、
`with open`、generator、使用 `nonlocal` 的巢狀函式、lambda 與 `Callable`；泛型函式、
`Protocol` 與聯集型別參數（依引數型別分別編譯）；`math`、`collections`、`heapq`、
`bisect`、`itertools`、`functools`、`random`（同一個 seed 產生和 CPython 一樣的數字）、
`re`、`sys`、`time`、`json.dumps`、`os.path`。

一些寫法翻譯後的樣子（更多在 [docs/cases](docs/cases/README.md)）：

| Python | Rust |
|---|---|
| `x: T \| None`、`if x is not None:` | `Option<T>`、`if let Some(x) = &x` |
| `node.next = self.head; self.head = node` | `node.next = self.head.take();` |
| 只讀取的區域變數（`cur = cur.next`） | 參考：`let mut cur = &self.head;` |
| `item = self.items[k]; item.qty -= 1` | `let item = &mut self.items[k];` |
| 有子類別的類別 | 列舉已建立的類別的 `enum`，方法用 `match` 分派 |
| 呼叫者會接住的 `raise` | 只在那條路徑上回傳 `Result<T, PyErr>`，使用 `?` |
| 使用 `seen` 和 `nonlocal count` 的巢狀函式 | `fn dfs(.., seen: &mut HashSet<..>, count: &mut i64)`，每次呼叫時傳入 |
| `def first[T](xs: list[T]) -> T` | `first_int(xs: &[i64])`、`first_str(xs: &[String])`，每種引數型別一份 |
| `def gen(n) -> Iterator[int]: ... yield i` | `pyrt::Gen<i64>`：函式本體和使用端嚴格輪流執行 |
| `@shared class Node` | `struct Node(Rc<RefCell<NodeData>>)`，加上 getter 和 setter |

與 CPython 已知的差異：`int` 是 64 位元（超出範圍會引發 `OverflowError`）；set 的走訪
順序是插入順序（見 [TODO.md](TODO.md)）。

## 速度

`python3 bench/run.py`，CPython 3.14 與翻譯後的程式比較（Apple silicon）：

| 程式 | CPython | Rust | |
|---|---:|---:|---:|
| nbody（浮點數物件） | 1.21 s | 0.012 s | 101 倍 |
| matmul（浮點數串列） | 0.47 s | 0.013 s | 38 倍 |
| fib（遞迴） | 0.26 s | 0.013 s | 20 倍 |
| sieve | 0.48 s | 0.027 s | 18 倍 |
| graph（用 deque 的 BFS） | 0.14 s | 0.017 s | 8 倍 |
| objects（dataclass、排序、dict） | 0.59 s | 0.10 s | 6 倍 |
| printing | 0.21 s | 0.056 s | 4 倍 |
| strings、wordfreq | | | 2 倍 |

## 所有權標註

參數模式是推導出來的。要固定某個參數的模式就加上標註（`python/rust` 模組讓這些標註在
CPython 下也能執行；把該目錄加進 `PYTHONPATH`）：

```python
from rust import Ref, Mut, Own

def fill(xs: Mut[list[int]], n: int) -> None: ...   # &mut Vec<i64>
def keep(xs: Own[list[int]]) -> None: ...           # Vec<i64>
```

程式碼和標註矛盾時（例如修改 `Ref[...]`）會報錯。

## Rust crate

`from rust.CRATE import Name` 使用 Rust crate。宣告來自 stub 檔 `CRATE.pyi`，依序在
`./stubs`、`PY2RS_STUBS` 和 py2rs 附帶的 stub（`stubs/regex.pyi`）中尋找；該 crate
會成為產生出來的專案的 Cargo 相依套件。

```python
from rust.regex import Regex

date = Regex.new(r"(\d{4})-(\d{2})-(\d{2})")      # Result → 無效時引發 ValueError
print(date.replace_all("on 2024-05-17", "$3/$2/$1"))
```

stub 是有型別宣告的 Python 檔，每個宣告對應一個 Rust 路徑或運算式範本
（`@rust("{self}.find({0}).map(|m| m.as_str().to_string())")`）。`py2rs stub
CRATE[@VERSION]` 會從 crate 的 rustdoc JSON 產生 stub。`from rust import inline` 是
逃生口：`inline("xs.sort_unstable();")`、`inline[int]("n.count_ones() as i64")`。

## Python 擴充模組

`py2rs pyo3 mod.py` 建置出 `mod.so`，Python 可以像原本的模組一樣 import。模組層級的
函式會被匯出（引數與回傳值可以是 int、float、bool、str、list、tuple、dict 和
`None`）；函式引發的例外會在 Python 中以相同類別引發；`print` 寫到 Python 的
`sys.stdout`。`tests/pyo3/run.sh` 檢查 import 這個模組印出的內容和 import 原本的
Python 檔相同。

## 測試

`py2rs test [檔案|目錄]`（預設 `tests/`）翻譯每個程式，全部放進一個 Cargo 套件建置，
再和 CPython（`PY2RS_PYTHON`，否則 python3.14 / 3.13 / 3.12）比較標準輸出、結束碼和
例外型別。有 `NAME.in` 時當作輸入。第一行是 `# error: TEXT` 的程式必須被拒絕，而且
錯誤訊息要包含 TEXT（`tests/err/`）。測試集約有 240 個程式和 45 個拒絕案例；
`py2rs fuzz` 另外產生隨機程式。

## 原始碼結構

- `src/front.rs`（及 `src/front/`）：ruff 剖析的 AST → 有型別的 IR（`src/ir.rs`）；
  標準函式庫、`re`、運算子方法、巢狀函式、`match`、泛型
- `src/own.rs`：所有權推導、活性分析、共享與走訪檢查、參考
- `src/exc.rs`：哪些例外會在哪裡被接住，因而決定哪些函式回傳 `Result`
- `src/gen.rs`（及 `src/gen/`）：IR → Rust
- `src/modules.rs`：多檔案程式；`src/pyo3.rs`：擴充模組
- `pyrt/`：執行期函式庫：Python 的輸出格式、格式規格、`//` `%` `**` 語意、以字元為
  單位的 `str` 方法、例外、標準函式庫、generator、緩衝輸出
