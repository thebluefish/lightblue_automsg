use lightblue_macro::{input, inputs};

#[inputs(MyProtocol)]
pub enum Inputs {
    Direction(Direction),
    Delete,
    Spawn,
    None,
}

#[input]
#[derive(Debug, PartialEq, Eq)]
pub struct Direction {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
}