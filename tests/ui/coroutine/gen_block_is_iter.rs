//@ revisions: next old
//@compile-flags: --edition 2024 -Zunstable-options
//@[next] compile-flags: -Znext-solver
//@ check-pass
#![feature(gen_blocks)]

fn foo() -> impl Iterator<Item = u32> {
    gen { yield 42 }
}

fn bar() -> impl Iterator<Item = i64> {
    gen { yield 42 }
}

fn baz() -> impl Iterator<Item = i32> {
    gen { yield 42 }
}

fn main() {}
