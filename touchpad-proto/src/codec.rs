use super::proto::{self, v1::wrapper::Payload};
use anyhow::{Result, anyhow};
use prost::Message;
use std::any::Any;
use std::io::Read;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, error};

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

/// Encode a protobuf message into a wrapper message with a length prefix.
pub fn wrap_with_prefix<M: Message + 'static>(msg: &M) -> Result<Vec<u8>> {
    let data = wrap(msg)?;
    return Ok(varint::encode_with_length_prefix(&data));
}

pub mod varint {
    use tokio::io::AsyncReadExt;

    use super::*;

    pub fn encode_with_length_prefix(data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut length = data.len() as u32;

        while length >= 0x80 {
            result.push(((length & 0x7F) as u8) | 0x80);
            length >>= 7;
        }
        result.push(length as u8);
        result.extend_from_slice(data);
        result
    }

    pub fn read_varint<R: Read>(reader: &mut R) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0;
        let mut buffer = [0u8; 1];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                return Err(anyhow!("Unexpected end of stream while reading varint"));
            }

            let byte = buffer[0];
            result |= ((byte & 0x7F) as u32) << shift;

            if shift > 28 {
                return Err(anyhow!("Varint too long (maximum 5 bytes)"));
            }

            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
        }

        Ok(result)
    }

    pub async fn read_varint_async<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0;
        let mut buffer = [0u8; 1];
        let mut byte_count = 0;

        debug!("开始读取varint...");
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            byte_count += 1;
            debug!(
                "读取到字节[{}]: 0x{:02X}, 读取字节数: {}",
                byte_count, buffer[0], bytes_read
            );

            if bytes_read == 0 {
                error!("流意外结束，已读取{}个varint字节", byte_count - 1);
                return Err(anyhow!("Unexpected end of stream while reading varint"));
            }

            let byte = buffer[0];
            result |= ((byte & 0x7F) as u32) << shift;
            debug!("当前varint结果: 0x{:X}, shift: {}", result, shift);

            if shift > 28 {
                return Err(anyhow!("Varint too long (maximum 5 bytes)"));
            }

            if (byte & 0x80) == 0 {
                debug!("varint读取完成，最终值: {} (0x{:X})", result, result);
                break;
            }
            shift += 7;
        }

        Ok(result)
    }

    pub fn read_exact_bytes<R: Read>(reader: &mut R, length: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; length];
        let mut total_read = 0;

        while total_read < length {
            let bytes_read = reader.read(&mut buffer[total_read..])?;
            if bytes_read == 0 {
                return Err(anyhow!(
                    "Unexpected end of stream while reading message bytes"
                ));
            }
            total_read += bytes_read;
        }

        Ok(buffer)
    }

    pub async fn read_exact_bytes_async<R: AsyncRead + Unpin>(
        reader: &mut R,
        length: usize,
    ) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; length];
        let mut total_read = 0;

        while total_read < length {
            let bytes_read = reader.read(&mut buffer[total_read..]).await?;
            if bytes_read == 0 {
                return Err(anyhow!(
                    "Unexpected end of stream while reading message bytes"
                ));
            }
            total_read += bytes_read;
        }

        Ok(buffer)
    }

    pub async fn read_message_with_length_prefix<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<Vec<u8>> {
        debug!("开始读取消息长度前缀...");
        let message_length = read_varint_async(reader).await?;
        debug!("读取到消息长度: {}", message_length);

        if message_length == 0 || message_length > 4096 {
            return Err(anyhow!("Invalid message length: {}", message_length));
        }

        debug!("开始读取{}字节的消息内容...", message_length);
        let message_bytes = read_exact_bytes_async(reader, message_length as usize).await?;
        debug!("成功读取{}字节的消息", message_bytes.len());

        Ok(message_bytes)
    }

    pub fn read_message_with_length_prefix_sync<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
        let message_length = read_varint(reader)?;

        if message_length == 0 || message_length > 4096 {
            return Err(anyhow!("Invalid message length: {}", message_length));
        }

        read_exact_bytes(reader, message_length as usize)
    }

    pub const MAX_MESSAGE_LENGTH: u32 = 4096;

    pub fn is_valid_message_length(length: u32) -> bool {
        length > 0 && length <= MAX_MESSAGE_LENGTH
    }

    pub fn set_max_message_length(_max_length: u32) {
        tracing::warn!("Dynamic message length setting not implemented");
    }
}

pub struct ProtoStream {
    reader: Box<dyn AsyncRead + Unpin + Send>,
    writer: Box<dyn AsyncWrite + Unpin + Send>,
}

impl From<TcpStream> for ProtoStream {
    fn from(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        ProtoStream {
            reader: Box::new(reader),
            writer: Box::new(writer),
        }
    }
}

impl ProtoStream {
    pub fn new(
        reader: Box<dyn AsyncRead + Unpin + Send>,
        writer: Box<dyn AsyncWrite + Unpin + Send>,
    ) -> Self {
        ProtoStream { reader, writer }
    }

    pub async fn send_message<M: Message + 'static>(&mut self, msg: &M) -> Result<()> {
        let data = varint::encode_with_length_prefix(&wrap(msg)?);
        self.writer.write_all(&data).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<Payload> {
        let data = varint::read_message_with_length_prefix(&mut self.reader).await?;
        let response = dewrap(&data)?;
        Ok(response)
    }
}
