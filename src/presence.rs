//! RustDesk OnlineRequest/OnlineResponse on the hbbs NAT TCP port (21115).
//! Wire fields and bit ordering follow hbb_common/protos/rendezvous.proto and
//! rustdesk-server's handle_online_request. This never initiates a remote session.

use prost::Message;
use std::{collections::HashMap, io, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

#[derive(Clone, PartialEq, Message)]
struct OnlineRequest {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, repeated, tag = "2")]
    peers: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
struct OnlineResponse {
    #[prost(bytes = "vec", tag = "1")]
    states: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
struct Envelope {
    #[prost(message, optional, tag = "23")]
    request: Option<OnlineRequest>,
    #[prost(message, optional, tag = "24")]
    response: Option<OnlineResponse>,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn frame(payload: &[u8]) -> io::Result<Vec<u8>> {
    if payload.len() > 16 * 1024 {
        return Err(invalid("Presence request too large"));
    }
    let width = if payload.len() <= 0x3f {
        1
    } else if payload.len() <= 0x3fff {
        2
    } else {
        3
    };
    let header = ((payload.len() as u32) << 2) | (width - 1);
    let mut result = header.to_le_bytes()[..width as usize].to_vec();
    result.extend_from_slice(payload);
    Ok(result)
}

async fn read_frame(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header[..1]).await?;
    let width = usize::from(header[0] & 3) + 1;
    stream.read_exact(&mut header[1..width]).await?;
    let size = (u32::from_le_bytes(header) >> 2) as usize;
    // A page has at most 100 IDs: the expected response is only a few bytes.
    if size == 0 || size > 4096 {
        return Err(invalid("Invalid presence response size"));
    }
    let mut payload = vec![0u8; size];
    stream.read_exact(&mut payload).await?;
    Ok(payload)
}

fn decode(payload: &[u8], count: usize) -> io::Result<Vec<bool>> {
    let envelope = Envelope::decode(payload).map_err(|_| invalid("Invalid presence protobuf"))?;
    let response = envelope
        .response
        .ok_or_else(|| invalid("Missing online response"))?;
    if envelope.request.is_some() || response.states.len() != count.div_ceil(8) {
        return Err(invalid("Presence bitmap does not match the request"));
    }
    Ok((0..count)
        .map(|index| response.states[index / 8] & (1 << (7 - index % 8)) != 0)
        .collect())
}

/// A saved admin setting overrides deployment defaults, without hardcoded IPs.
pub async fn server(state: &crate::state::AppState) -> Result<(String, bool), sqlx::Error> {
    let saved: Option<String> =
        sqlx::query_scalar("SELECT value FROM server_settings WHERE key = 'rendezvous_server'")
            .fetch_optional(&state.db)
            .await?;
    let overridden = saved.is_some();
    Ok((
        saved.unwrap_or_else(|| state.config.rendezvous_server.clone()),
        overridden,
    ))
}

pub async fn query(endpoint: &str, ids: &[String]) -> io::Result<HashMap<String, bool>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    if ids.len() > 100 || ids.iter().any(|id| id.is_empty() || id.len() > 128) {
        return Err(invalid("Invalid presence query IDs"));
    }
    let result = timeout(Duration::from_secs(3), async {
        let request = Envelope {
            request: Some(OnlineRequest {
                id: "rustbook".into(),
                peers: ids.to_vec(),
            }),
            response: None,
        };
        let mut stream = TcpStream::connect(endpoint).await?;
        stream.write_all(&frame(&request.encode_to_vec())?).await?;
        let payload = read_frame(&mut stream).await?;
        let states = decode(&payload, ids.len())?;
        Ok::<_, io::Error>(ids.iter().cloned().zip(states).collect())
    })
    .await;
    result.unwrap_or_else(|_| {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "hbbs presence query timed out",
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn decodes_protocol_bitmap_across_byte_boundary() {
        let payload = Envelope {
            request: None,
            response: Some(OnlineResponse {
                states: vec![0x81, 0x80],
            }),
        }
        .encode_to_vec();
        assert_eq!(
            decode(&payload, 9).unwrap(),
            vec![true, false, false, false, false, false, false, true, true]
        );
        assert!(decode(&payload, 1).is_err());
        assert!(decode(&[0xff, 0xff], 1).is_err());
        assert!(decode(
            &Envelope {
                request: None,
                response: None
            }
            .encode_to_vec(),
            1
        )
        .is_err());
    }

    #[tokio::test]
    async fn queries_hbbs_with_native_framing_and_accepts_fragmented_reply() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let payload = read_frame(&mut stream).await.unwrap();
            let request = Envelope::decode(payload.as_slice())
                .unwrap()
                .request
                .unwrap();
            assert_eq!(request.peers, vec!["100", "200"]);
            let reply = frame(
                &Envelope {
                    request: None,
                    response: Some(OnlineResponse { states: vec![0x80] }),
                }
                .encode_to_vec(),
            )
            .unwrap();
            for byte in reply {
                stream.write_all(&[byte]).await.unwrap();
            }
        });
        let states = query(&endpoint, &["100".into(), "200".into()])
            .await
            .unwrap();
        assert!(states["100"]);
        assert!(!states["200"]);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn oversized_response_fails_without_allocating_advertised_payload() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            read_frame(&mut stream).await.unwrap();
            stream.write_all(&[0xff, 0xff, 0xff, 0xff]).await.unwrap();
        });
        assert!(query(&endpoint, &["100".into()]).await.is_err());
        server.await.unwrap();
    }
}
