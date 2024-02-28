use lightblue_macro::*;

#[message(MyProtocol)]
#[derive(Clone, Debug, PartialEq)]
pub struct Bar(pub bool);
