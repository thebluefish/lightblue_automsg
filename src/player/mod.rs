pub(crate) mod net;

use bevy::prelude::*;
use lightblue_macro::*;
use lightyear::prelude::*;

#[input(MyProtocol, sync(once))]
#[derive(Debug, PartialEq, Eq)]
pub struct Direction {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
}

#[component(MyProtocol, sync(once))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);

#[component(MyProtocol, sync(once))]
#[derive(Clone, Debug, PartialEq)]
pub struct AltColor(pub(crate) Color);
