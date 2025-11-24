use anyhow::{Result, anyhow};
use prost::Message;
use std::any::Any;
use touchpad_proto::proto::{self, v1::wrapper::Payload};

pub fn wrap<M: Message + 'static>(msg: &M) -> Result<Vec<u8>> {
    use proto::v1::{Wrapper, wrapper::Payload};

    let wrapper = Wrapper {
        payload: Some(
            if let Some(pb) = (msg as &dyn Any).downcast_ref::<proto::v1::Welcome>() {
                Payload::Welcome(pb.clone())
            } else if let Some(pb) = (msg as &dyn Any).downcast_ref::<proto::v1::Reject>() {
                Payload::Reject(pb.clone())
            } else if let Some(pb) = (msg as &dyn Any).downcast_ref::<proto::v1::HeartBeat>() {
                Payload::HeartBeat(pb.clone())
            } else if let Some(pb) = (msg as &dyn Any).downcast_ref::<proto::v1::TouchPacket>() {
                Payload::TouchPacket(pb.clone())
            } else if let Some(pb) =
                (msg as &dyn Any).downcast_ref::<proto::v1::DiscoverValidation>()
            {
                Payload::DiscoverValidation(pb.clone())
            } else {
                anyhow::bail!("unsupported message type")
            },
        ),
    };
    Ok(wrapper.encode_to_vec())
}

/// Decode a wrapper message into a protobuf message.
pub fn dewrap(buf: &[u8]) -> Result<Payload> {
    use proto::v1::Wrapper;

    let wrapper = Wrapper::decode(buf)?;
    if let Some(payload) = wrapper.payload {
        Ok(payload)
    } else {
        Err(anyhow!("The data payload is None"))
    }
}
