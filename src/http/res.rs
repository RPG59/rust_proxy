use std::collections::HashMap;

use log::{debug, error, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
};

pub struct RpgxResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl RpgxResponse {
    pub fn new(status: u16) -> Self {
        RpgxResponse {
            status,
            headers: [(
                "content-type".to_string(),
                "text/html; charset=utf-8".to_string(),
            )]
            .iter()
            .cloned()
            .collect(),
            body: Vec::new(),
        }
    }

    pub fn make_internal_error() -> Self {
        let headers = [(
            "content-type".to_string(),
            "text/html; charset=utf-8".to_string(),
        )]
        .iter()
        .cloned()
        .collect();

        let body = r#"
            <html>
                <head><title>500 Not Found</title></head>
                <body bgcolor="white">
                    <center><h1>500 Internal Server Error</h1></center>
                    <hr/>
                </body>
            </html>
        "#;

        RpgxResponse {
            status: 500,
            headers,
            body: body.into(),
        }
    }

    pub async fn send(&self, stream: &mut TcpStream) {
        let headers: String = self
            .headers
            .clone()
            .into_iter()
            .map(|x| format!("{}:{}\r\n", x.0, x.1))
            .collect();

        let data = format!(
            "HTTP/1.1 {} {}\r\n{}\r\n",
            self.status,
            self.get_reason(),
            headers
        );

        let buffer = [data.as_bytes().to_vec(), self.body.clone()].concat();
        let res = stream.write_all(&buffer).await;

        if res.is_err() {
            warn!("Failed to send data. Error: {}", res.err().unwrap());
        }

        let shutdown_status = stream.shutdown().await;

        if shutdown_status.is_err() {
            warn!(
                "Failed to close connection. Error: {}",
                shutdown_status.err().unwrap()
            )
        }
    }

    fn get_reason(&self) -> String {
        match self.status {
            200 => "Ok",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
        .to_string()
    }
}
