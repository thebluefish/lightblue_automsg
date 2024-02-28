use std::ops::Mul;
use bevy::prelude::*;
use derive_more::{Add, Mul};
use lightblue_macro::*;
use lightyear::prelude::*;

#[component(MyProtocol, sync(once))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerId(ClientId);

#[component(MyProtocol, sync(full))]
#[derive(Clone, Debug, PartialEq, Deref, DerefMut, Add, Mul)]
pub struct PlayerPosition(Vec2);

impl Mul<f32> for &PlayerPosition {
    type Output = PlayerPosition;

    fn mul(self, rhs: f32) -> Self::Output {
        PlayerPosition(self.0 * rhs)
    }
}