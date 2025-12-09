use once_cell::sync::Lazy;
use prost_reflect::DescriptorPool;

static DESCRIPTOR_POOL: Lazy<DescriptorPool> = Lazy::new(|| {
    DescriptorPool::decode(include_bytes!(concat!(env!("OUT_DIR"), "/touchpad.v1.bin")).as_ref())
        .unwrap()
});

pub mod proto {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/touchpad.v1.rs"));
    }
}
