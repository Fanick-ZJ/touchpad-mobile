use anyhow::{Result, anyhow};
use prost::Message;
use prost_reflect::ReflectMessage;
use prost_types::Any as ProstAny;
use quinn::RecvStream;
use std::any::Any;
use touchpad_proto::proto;

pub async fn wrapper(pb: &dyn Any) -> Result<Vec<u8>> {
    let (type_url, value, proto_id) = if let Some(pb) = pb.downcast_ref::<proto::v1::Welcome>() {
        (
            format!("type.googleapis.com/{}", pb.descriptor().full_name()),
            pb.encode_to_vec(),
            proto::v1::ProtoId::HandShake,
        )
    } else if let Some(pb) = pb.downcast_ref::<proto::v1::Heartbeat>() {
        (
            format!("type.googleapis.com/{}", pb.descriptor().full_name()),
            pb.encode_to_vec(),
            proto::v1::ProtoId::HeartBeat,
        )
    } else if let Some(pb) = pb.downcast_ref::<proto::v1::TouchPacket>() {
        (
            format!("type.googleapis.com/{}", pb.descriptor().full_name()),
            pb.encode_to_vec(),
            proto::v1::ProtoId::TouchPacket,
        )
    } else {
        return Err(anyhow!(format!("Invalid message type: {:?}", pb)));
    };
    let wrapper = proto::v1::Wrapper {
        proto_id: proto_id as i32,
        payload: Some(ProstAny {
            type_url: type_url,
            value: value,
        }),
    };
    let wrapper_buf = wrapper.encode_to_vec();
    Ok(wrapper_buf)
}

pub async fn dewrapper(recv_stream: &mut RecvStream) -> Result<(i32, ProstAny)> {
    let mut buf = Vec::with_capacity(1024);
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        match recv_stream.read(&mut buf).await {
            Ok(Some(length)) => {
                bytes.extend(&buf[..length]);
            }
            Ok(None) => break,
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    let wrapper = proto::v1::Wrapper::decode(&bytes[..])?;
    if let None = wrapper.payload {
        return Err(anyhow!("The data payload is None"));
    } else {
        Ok((wrapper.proto_id, wrapper.payload.unwrap()))
    }
}
