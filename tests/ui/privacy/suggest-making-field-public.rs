//@ run-rustfix
pub mod a {
    pub struct A(pub(self)String);
}

mod b {
    use crate::a::A;
    pub fn x() {
        A("".into()); //~ ERROR cannot initialize a tuple struct which contains private fields
    }
}
fn main() {
    a::A("a".into()); //~ ERROR tuple struct constructor `A` is private
    b::x();
}
