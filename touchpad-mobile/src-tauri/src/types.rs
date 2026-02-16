use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum TouchPointStatus {
    Add = 0,
    Move = 1,
    Leave = 2,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct FrontTouchPoint {
    pub tracking_id: i32,
    pub status: TouchPointStatus,
    pub x: u32,
    pub y: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct FrontTuneSetting {
    pub sensitivity: f32,
    pub invert_x: bool,
    pub invert_y: bool,
}
