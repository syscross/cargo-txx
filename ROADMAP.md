
# ROADMAP

Things TXX doesn't do yet, but should eventually. Nothing here is
implemented — this is a wish list plus rough notes on how each one
might get built.

---

## Wished-for syntax

### Real nested modules

```
module shapes {
    module circle {
        struct Circle { r: f64, }
    }
}
```
Right now C++ modules only fake hierarchy with dots in the name
(`shapes.circle`), and re-exports have to be written by hand. TXX
should generate all that boilerplate automatically from real nesting.

*Rough plan*: parse the nested `module { }` blocks, flatten each one
into a standard `export module shapes.circle;` file, then
auto-generate the `export import` lines in the parent module.

---

### `let` without a type, when it's obvious

```
let a = Hello { name: "TXX" };
```
Right now every `let` needs an explicit type. If the right-hand side
already says the type, TXX shouldn't make you repeat it.

*Rough plan*: only allow this when the right side is a struct literal
or a call with a known return type — skip full type inference for now.

---

### Struct literals (no more empty `let` + manual field assignment)

```
let a: Hello = Hello { name: "TXX" };
```
instead of

```
let a: Hello;
a.name = "TXX";
```
This alone would make the "uninitialized value" check almost
unnecessary — the struct is either fully built in one expression, or
it doesn't compile.

*Rough plan*: this is probably the single highest-value feature to
build next, since it fixes the uninit problem at the syntax level
instead of only catching it after the fact.

---

### `import std;` that's actually safe on every compiler

Right now this just becomes `#include <iostream>` etc. behind the
scenes, because real C++ modules are still too unreliable across
GCC/Clang/MSVC (see cpp-mod-mgr's 2026 report — real modules still
fail on all three, in different ways).

*Rough plan*: keep a small table of "which compiler + version
actually handles `import std;` today" and only use real modules when
it's known to work. Fall back to headers otherwise. Revisit this
table periodically as compiler support improves.

---

## Wished-for checks

### Catching multiple inheritance with data

```
struct Widget : Base1, Base2 {
    c: i32,
}
```
If `Base1` and `Base2` both have fields, this should be a hard error
— C++ allows it, Rust has no equivalent, and it's rarely what anyone
actually wants.

*Rough plan*: at struct-parse time, check if any base struct (or its
bases) has fields. If two or more do, reject.

---

### Warning when code won't survive `export-rs`

Even before `export-rs` exists, TXX could start flagging patterns
that are known to not map to Rust — raw pointers with unclear
ownership, `friend`, C-style casts. Warn now, so by the time
`export-rs` exists there's less to fix.

*Rough plan*: a `--future-rust` flag that runs a stricter check pass,
off by default.

---

## Wished-for tooling

### `cargo txx build` on a project with no `Cargo.toml`, just `CMakeLists.txt`

The idea: `cargo-txx` notices there's no `Cargo.toml`, finds a
`CMakeLists.txt` instead, and just shells out to `cmake -B build &&
cmake --build build`. Zero setup cost to try the tool on an existing
C++ project.

*Rough plan*: check for `Cargo.toml` first; if missing, look for
`CMakeLists.txt` and proxy `build`/`run`/`test` to the equivalent
CMake/CTest commands.

---

### `export-rs`

Turn a `.txx` file into an actual `.rs` file, once enough of the
"wished-for checks" above are in place to make the output plausible
Rust rather than C++ wearing a trench coat.

*Rough plan*: start with the "brutal" version — wrap anything
ownership-unclear in `Rc<RefCell<T>>` and raw pointers in `unsafe`,
just to get something that compiles with `rustc`. Refine later.

---

## Not planning to build (for now)

Keeping this here so the list above doesn't quietly grow to include
these:

- A real standalone compiler (this stays a source-to-source tool on
  top of existing C++ compilers)
- Full Rust borrow-checker equivalence — `export-rs` output doesn't
  need to be idiomatic Rust on day one, just Rust that compiles
