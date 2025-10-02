pub mod touchpad {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/touchpad.v1.rs"));
    }
}
