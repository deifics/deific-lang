<h1 align="center">Deific</h1>

<p align="center">
  <strong>Python's syntax, transpiled to C++. C++ performance for competitive programming.</strong>
</p>

<p align="center">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg">
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange.svg">
  <img alt="Target: C++17" src="https://img.shields.io/badge/target-C%2B%2B17-00599C.svg">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg">
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-green.svg">
</p>

---

Deific looks like Python but compiles to a native binary through C++. Write solutions in readable Python-style syntax, ship code that runs at full C++ speed. That tradeoff is the entire point: when Python ergonomics and C++ speed conflict, **speed wins, every time.**

```python
func main():
    n = int(input())
    a = read_ints(n)
    a.sort()
    total = 0
    for i in range(n):
        total = total + a[i]
    print(total)
    print(a)
```

```
$ deific run examples/sort.df
```

---

## The Three Deliberate Breaks from Python

Each is a place where true Python behavior and C++ performance are in direct conflict. Deific always picks performance:

| Area | Python | Deific | Why |
|------|--------|--------|-----|
| **Numbers** | Arbitrary precision | `int` = 64-bit `long long`; `bigint` is opt-in | Bignum is a function call per op; competitive programming thinks in `long long` |
| **Assignment** | Reference + GC | Value semantics (copy); `ref` for in-place | A GC is the thing that makes Python slow |
| **Strings** | Unicode codepoints | Byte strings (`std::string`) | O(1) Unicode indexing hides real cost; competitive programming is ASCII |

---

## How It Works

```
.df source --lexer--> tokens --parser--> AST --emit--> .cpp --g++--> native binary
```

- **Local type inference** leans on C++ `auto` - you only annotate function signatures and empty containers.
- The emitted `.cpp` inlines a **buffered fast-I/O runtime**, so the generated file is self-contained and directly submittable to an online judge.
- Integer literals carry an `LL` suffix so `int` is 64-bit everywhere.
- `#line` directives in emitted C++ map g++ error messages back to `.df` source line numbers.

---

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable, GNU toolchain on Windows)
- `g++` with C++17 support - [MSYS2 ucrt64](https://www.msys2.org/) recommended on Windows

### Build from source

```powershell
# Windows: use the GNU toolchain (MSVC does not work)
cargo +stable-x86_64-pc-windows-gnu build --release
```

```bash
# Linux / macOS
cargo build --release
```

The compiled binary will be at `target/release/deific` (or `deific.exe` on Windows).

---

## CLI

```
deific emit  <file.df>           # print generated C++ to stdout (submittable)
deific build <file.df> [-o out]  # compile to a native binary via g++
deific run   <file.df>           # build to a temp binary and run it immediately
```

**Reading from a file on Windows** - avoid PowerShell pipes (they re-encode to UTF-16). Use a file redirect instead:

```powershell
cmd /c ".\solution.exe < input.txt"
```

---

## Language Reference

### Hello World

```python
func main():
    print("Hello World!")
```

### Variables and Type Inference

Local variables are inferred from assignment. You only need a type annotation for empty containers:

```python
func main():
    x = 42          # long long
    f = 3.14        # double
    s = "hello"     # std::string
    flag = True     # bool

    nums: list[int] = []       # must annotate empty list
    d: dict[str, int] = {}     # must annotate empty dict
```

### Types

| Deific | C++ | Notes |
|--------|-----|-------|
| `int` | `long long` | 64-bit signed, always |
| `float` | `double` | IEEE 754 double |
| `str` | `std::string` | Byte string |
| `bool` | `bool` | `True` / `False` |
| `bigint` | `__int128` | Opt-in, GCC only |
| `list[T]` | `std::vector<T>` | |
| `dict[K, V]` | `std::unordered_map<K, V>` | |
| `set[T]` | `std::unordered_set<T>` | |
| `tuple[A, B]` | `std::pair<A, B>` | |

### Arithmetic

```python
x = 10 + 3   # addition
x = 10 - 3   # subtraction
x = 10 * 3   # multiplication
x = 10 / 3   # float division
x = 10 // 3  # integer floor division
x = 10 % 3   # modulo
x = 2 ** 8   # power (256)
```

### Bitwise Operators

```python
a = x & y    # AND
a = x | y    # OR
a = x ^ y    # XOR
a = ~x       # NOT (bitwise complement)
a = x << 2   # left shift
a = x >> 2   # right shift
```

### Augmented Assignment

```python
x += 5
x -= 5
x *= 2
x //= 2
x %= 3
x **= 2
x &= 0xFF
x |= 0x01
x ^= 0x10
x <<= 1
x >>= 1
```

### Comparisons and Booleans

```python
a == b
a != b
a < b
a <= b
a > b
a >= b
a and b
a or b
not a
3 in nums       # membership test
3 not in nums
```

### Control Flow

```python
if x > 0:
    print("positive")
elif x == 0:
    print("zero")
else:
    print("negative")

while x > 0:
    x -= 1

for i in range(10):
    if i == 5:
        break
    if i % 2 == 0:
        continue
    print(i)

pass  # no-op placeholder
```

### Range

```python
range(n)              # 0 .. n-1
range(start, stop)    # start .. stop-1
range(start, stop, step)
```

### Functions

```python
func add(a: int, b: int) -> int:
    return a + b

func greet(name: str) -> str:
    return "Hello, " + name
```

#### Generic functions (C++ templates)

```python
func[T] identity(x: T) -> T:
    return x
```

#### Reference parameters (in-place mutation)

```python
func swap(a: ref int, b: ref int):
    a, b = b, a
```

### Lists

```python
nums = [1, 2, 3, 4, 5]
nums.append(6)
nums.pop()
nums.sort()
nums.reverse()
n = len(nums)
first = nums[0]
last = nums[-1]
slice = nums[1:4]
```

#### List comprehensions

```python
squares  = [x * x for x in nums]
evens    = [x for x in nums if x % 2 == 0]
matrix   = [i * j for i in range(5) for j in range(5)]
```

### Dictionaries

```python
d: dict[str, int] = {}
d["a"] = 1
d["b"] = 2

counts: dict[str, int] = {"apple": 3, "banana": 7}
exists = "apple" in counts
```

### Sets

```python
s: set[int] = {1, 2, 3}
s2 = {x * x for x in range(5)}
exists = 3 in s
```

### Strings

```python
s = "hello"
s2 = s.upper()
s3 = s.lower()
s4 = s.strip()
s5 = s.replace("hello", "world")
parts = s.split(" ")
joined = " ".join(parts)
found = s.startswith("he")
idx = s.find("ll")
n = len(s)
c = s[0]
sub = s[1:3]
```

### Tuple / Multi-Assignment

```python
a, b = 1, 2
a, b = b, a    # swap
```

### Built-in Functions

| Function | Description |
|----------|-------------|
| `print(...)` | Print space-separated values with newline |
| `input()` | Read a line from stdin |
| `int(x)` | Convert to 64-bit integer |
| `float(x)` | Convert to double |
| `str(x)` | Convert to string |
| `len(x)` | Length of list, string, dict, or set |
| `abs(x)` | Absolute value |
| `min(a, b)` / `min(lst)` | Minimum |
| `max(a, b)` / `max(lst)` | Maximum |
| `sum(lst)` | Sum of list |
| `sorted(lst)` | Return sorted copy |
| `reversed(lst)` | Return reversed copy |
| `enumerate(lst)` | Pairs of (index, value) |
| `zip(a, b)` | Pairs of corresponding elements |
| `chr(n)` | Integer to character |
| `ord(s)` | Character to integer |
| `gcd(a, b)` | Greatest common divisor |
| `lcm(a, b)` | Least common multiple |

### Competitive Programming I/O

For online judges with large input, use the fast buffered readers instead of `input()`:

| Function | Description |
|----------|-------------|
| `read_int()` | Read one integer from stdin |
| `read_ints(n)` | Read n integers, return `list[int]` |
| `read_float()` | Read one float from stdin |
| `read_floats(n)` | Read n floats, return `list[float]` |

```python
func main():
    n = read_int()
    a = read_ints(n)
    a.sort()
    for x in a:
        print(x)
```

### Big Integers

```python
func main():
    x: bigint = 170141183460469231731687303715884105727
    print(x)
```

`bigint` maps to `__int128` on GCC. All arithmetic operators work. Print is handled automatically.

---

## Examples

### Sort and sum

```python
func main():
    n = int(input())
    a = read_ints(n)
    a.sort()
    total = 0
    for i in range(n):
        total += a[i]
    print(total)
    print(a)
```

### Binary search

```python
func binary_search(a: list[int], target: int) -> int:
    lo = 0
    hi = len(a) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        if a[mid] == target:
            return mid
        elif a[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1

func main():
    n = read_int()
    a = read_ints(n)
    a.sort()
    q = read_int()
    for i in range(q):
        target = read_int()
        print(binary_search(a, target))
```

### Word frequency

```python
func main():
    n = int(input())
    counts: dict[str, int] = {}
    for i in range(n):
        word = input().strip()
        if word in counts:
            counts[word] += 1
        else:
            counts[word] = 1
    print(len(counts))
```

---

## Architecture

The compiler is a single Rust binary with zero external dependencies.

| File | Role |
|------|------|
| `src/main.rs` | CLI entry point; orchestrates the full pipeline |
| `src/lexer.rs` | Line-based tokenizer; emits `INDENT`/`DEDENT` for Python-style blocks |
| `src/parser.rs` | Recursive-descent + Pratt expression parser; produces the AST |
| `src/ast.rs` | AST node definitions |
| `src/emit.rs` | Walks the AST and generates C++; inlines the fast-I/O runtime |

### Adding a language feature

The typical change touches these files in order:

1. `lexer.rs` - add any new token or keyword
2. `parser.rs` - parse new syntax into an AST node
3. `ast.rs` - add the new node variant
4. `emit.rs` - emit C++ for the new node

---

## License

MIT - see [LICENSE](LICENSE) for details.
