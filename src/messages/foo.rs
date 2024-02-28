use lightblue_macro::*;

#[message(MyProtocol)]
#[derive(Clone, Debug, PartialEq)]
pub struct Foo(pub usize);
