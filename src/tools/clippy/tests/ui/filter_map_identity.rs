#![allow(unused_imports, clippy::needless_return, clippy::useless_vec)]
#![warn(clippy::filter_map_identity)]
#![feature(stmt_expr_attributes)]

use std::option::Option;
struct NonCopy;
use std::convert::identity;

fn non_copy_vec() -> Vec<Option<NonCopy>> {
    todo!()
}

fn copy_vec<T: Copy>() -> Vec<Option<T>> {
    todo!()
}

fn copy_vec_non_inferred() -> Vec<Option<i32>> {
    todo!()
}

fn opaque<T: Default>() -> impl IntoIterator<Item = Option<T>> {
    vec![Some(T::default())]
}

fn main() {
    {
        // into_iter
        copy_vec_non_inferred().into_iter().filter_map(|x| x);
        //~^ ERROR: use of
        copy_vec_non_inferred().into_iter().filter_map(std::convert::identity);
        //~^ ERROR: use of
        copy_vec_non_inferred().into_iter().filter_map(identity);
        //~^ ERROR: use of
        copy_vec_non_inferred().into_iter().filter_map(|x| return x);
        //~^ ERROR: use of
        copy_vec_non_inferred().into_iter().filter_map(|x| return x);
        //~^ ERROR: use of

        non_copy_vec().into_iter().filter_map(|x| x);
        //~^ ERROR: use of
        non_copy_vec().into_iter().filter_map(|x| x);
        //~^ ERROR: use of

        non_copy_vec().into_iter().filter_map(std::convert::identity);
        //~^ ERROR: use of
        non_copy_vec().into_iter().filter_map(identity);
        //~^ ERROR: use of
        non_copy_vec().into_iter().filter_map(|x| return x);
        //~^ ERROR: use of
        non_copy_vec().into_iter().filter_map(|x| return x);
        //~^ ERROR: use of

        copy_vec::<i32>().into_iter().filter_map(|x: Option<_>| x);
        //~^ ERROR: use of
        copy_vec::<i32>().into_iter().filter_map(|x: Option<_>| x);
        //~^ ERROR: use of
        copy_vec::<i32>().into_iter().filter_map(|x: Option<_>| return x);
        //~^ ERROR: use of
        copy_vec::<i32>().into_iter().filter_map(|x: Option<_>| return x);
        //~^ ERROR: use of

        // we are forced to pass the type in the call.
        copy_vec::<i32>().into_iter().filter_map(|x: Option<i32>| x);
        //~^ ERROR: use of
        copy_vec::<i32>().into_iter().filter_map(|x: Option<i32>| x);
        //~^ ERROR: use of
        copy_vec::<i32>().into_iter().filter_map(|x: Option<i32>| return x);
        //~^ ERROR: use of
        copy_vec::<i32>().into_iter().filter_map(|x: Option<i32>| return x);
        //~^ ERROR: use of
        #[rustfmt::skip]
            copy_vec::<i32>().into_iter().filter_map(|x: Option<i32>| -> Option<i32> {{ x }});
        //~^ ERROR: use of
        #[rustfmt::skip]
            copy_vec::<i32>().into_iter().filter_map(|x: Option<i32>| -> Option<i32> {{ return x }});
        //~^ ERROR: use of

        // note, the compiler requires that we pass the type to `opaque`. This is mostly for reference,
        // it behaves the same as copy_vec.
        opaque::<i32>().into_iter().filter_map(|x| x);
        //~^ ERROR: use of
    }
}

fn issue12653() -> impl Iterator<Item = u8> {
    [].into_iter().filter_map(|x| x)
    // No lint
}
