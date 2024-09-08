use http::Error;
use log::{debug, error, warn};
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use url::Url;

use super::{req::RpgxRequest, res::RpgxResponse};

struct Connection {
    creation_time_ms: u32,
    stream: TcpStream,
}

pub struct HttpClient {}

pub struct HttpClientError {
    message: String,
}

impl HttpClientError {
    pub fn new(message: String) -> Self {
        HttpClientError { message }
    }
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {}
    }

    pub async fn execute(&self, req: &RpgxRequest) -> Result<RpgxResponse, String> {
        let path = req.url.to_string();

        let stream_res = TcpStream::connect("localhost:3000").await;

        if stream_res.is_err() {
            let msg = format!(
                "Failed to connect to {}, Error: {}",
                path,
                stream_res.err().unwrap()
            );

            return Err(msg);
        }

        let mut stream = stream_res.unwrap();

        stream.write_all(req.to_vec().as_bytes()).await.unwrap();

        let mut buffer = Vec::new();
        let recv_size = stream.read_to_end(&mut buffer).await.unwrap();

        debug!("Received {} bytes", recv_size);

        stream.shutdown().await.unwrap();

        let buffer = &buffer[0..recv_size];

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Response::new(&mut headers);
        let body_offset = req.parse(&buffer).unwrap().unwrap();

        let mut response = RpgxResponse {
            status: req.code.unwrap(),
            headers: std::collections::HashMap::new(),
            body: Vec::new(),
        };

        for header in headers {
            response.headers.insert(
                header.name.to_string(),
                String::from_utf8_lossy(&header.value).to_string(),
            );
        }

        // response.body.clone_from_slice(buffer);
        response.body = buffer.to_vec();

        Ok(response)
    }

    // pub async fn get_connection(&mut self, url: &Url) -> Result<Arc<Connection>, std::io::Error> {
    //     let host = url.host().unwrap().to_string();

    //     match self.pool.get(&host) {
    //         Some(conn) => Ok(conn.clone()),
    //         _ => {
    //             let stream = TcpStream::connect(&host).await?;
    //             let conn = Arc::new(Connection {
    //                 creation_time_ms: 0,
    //                 stream,
    //             });

    //             self.set_connection(host, conn.clone());

    //             Ok(conn)
    //         }
    //     }
    // }
}
