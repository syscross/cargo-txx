
# cargo-txx

A small syntax layer on top of C++. It compiles `.txx` files by turning
them into plain C++, then building that with clang++ or g++.

## What works now

### Syntax changes

- **No semicolon after struct/class**
  ```
  struct Point {
      x: i32,
  }
  ```
  Normal C++ needs a `;` after `struct { ... }`. That rule is old — it
  was for a C-style pattern nobody really uses anymore. TXX drops it.

- **Field order: `name: type`**
  ```
  name: std::string,
  ```
  Instead of C++'s `std::string name;`, TXX writes the name first.

- **`fn` keyword**
  ```
  fn main() { ... }
  fn Hello::hello() { ... }
  ```
  `fn` just becomes `auto` (or `int` for `main`). This reuses normal
  C++ syntax for defining a method outside its struct — TXX adds the
  matching declaration inside the struct automatically.

- **`main()` returns 0 automatically**
  If there's no `return` in `main`, one is added for you.

- **Automatic semicolons (early version)**
  Inside a function body, if a line doesn't end in `;`, `{`, `}`, and
  isn't a comment, a semicolon gets added.
  ⚠️ Only works for single-line statements right now.

### Compile-time checks

These stop compilation if broken:

- **Fields must use `self.`**
  ```
  error: bare field access `name` — did you mean `self.name`?
  ```
  Using a field name without `self.` inside a method is an error.

- **No using uninitialized values**
  ```
  error: use of possibly-uninitialized value `a` — missing field(s): `name`
  ```
  A variable declared with `let a: Type;` can't have its methods
  called until every field has been assigned.

## Known limits

- `//` inside a string literal (like `"http://..."`) is wrongly
  treated as a comment
- Automatic semicolons break on multi-line statements
- If two structs share a name, only the first one is recognized
- Method signatures with nested parentheses (like
  `std::function<void()>`) fail to parse
- This is regex + brace-matching, not a real parser — a real parser
  is planned later

## Planned (not built yet)

- Real module nesting (`module shapes { module circle { ... } }`)
  with automatic re-exports
- `export-rs`: turning TXX code into a real Rust crate
- Moving lint rules out of the code and into a config file
- Running CMake projects directly, even with no `Cargo.toml`

## Usage

```bash
cargo run -- path/to/file.txx
```
This writes the converted `.cpp` and the compiled binary into
`target/txx/`, then runs it.
