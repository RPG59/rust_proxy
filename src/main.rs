use core::str;
use std::{
    collections::{self, HashMap},
    error,
    fmt::{format, Pointer},
    io::{Read, Write},
    ptr::null,
};

use log::{debug, error, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
};

use env_logger;
use httparse::{self, Header};
use reqwest;

mod config;
use config::Config;
use config::Location;

mod http;
use http::client::HttpClient;
use http::req::RpgxRequest;
use http::res::RpgxResponse;
use url::{ParseError, Url};

// async fn test() {
//     let res = reqwest::get("http://127.0.0.1:3000").await.unwrap();
//     // let mut res = surf::get("https://httpbin.org/get").await.unwrap();
//     // dbg!(res.body_string().await.unwrap());
// }
/*
   TODO:
       - Config Lang
           -- server_name
           -- location
           -- proxy_set_header
           -- add_header
           -- proxy_pass
           -- keep alive
           -- transfer-encoding chunked
*/

// struct Response<'a> {
//     headers: Vec<httparse::Header<'a>>,
//     body: Vec<u8>,
// }

// async fn make_request<'a>(req: &httparse::Request<'a, 'a>) -> Box<Response<'a>> {
//     let http_header = format!("{} {} HTTP/1.1", req.method.unwrap(), req.path.unwrap());
//     let headers = req
//         .headers
//         .to_vec()
//         .into_iter()
//         .map(|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap()))
//         .collect();

//     let req_str = vec![http_header, headers, "\r\n".to_string()].join("\r\n");

//     println!("HTTP Request String:\n{}", req_str);

//     let mut stream: TcpStream = TcpStream::connect("127.0.0.1:3000").await.unwrap();

//     stream.write_all(req_str.as_bytes()).await.unwrap();

//     // let mut buffer = Vec::new();
//     // let recv_size = stream.read_to_end(&mut buffer).await.unwrap();

//     let mut buffer_raw = [0; 1024 * 8];
//     let recv_size = stream.read(&mut buffer_raw).await.unwrap();
//     let buffer = &buffer_raw[0..recv_size];

//     println!("Reveive {} bytes", recv_size);

//     // let mut headers = Box::new([httparse::EMPTY_HEADER; 64]);
//     let mut headers_slice = [httparse::EMPTY_HEADER; 64];
//     let mut req = httparse::Response::new(&mut headers_slice);
//     let body_offset = req.parse(&buffer).unwrap().unwrap();

//     // for header in headers {
//     //     println!(
//     //         "Header: {}:{}",
//     //         header.name,
//     //         String::from_utf8_lossy(header.value)
//     //     )
//     // }

//     // println!(
//     //     "Headers:\n{}",
//     //     String::from_utf8_lossy(&buffer[0..body_offset])
//     // );

//     let mut body = Vec::with_capacity(recv_size - body_offset);
//     body.clone_from_slice(&buffer[body_offset..]);

//     ////aaaaaaaaaaaaaaaaaa
//     let mut headers = Vec::with_capacity(req.headers.len());

//     for header in req.headers {
//         let test = header.clone();

//         headers.push(test);
//     }
//     ///////

//     Box::new(Response {
//         headers: vec![],
//         body,
//     })

//     // println!("Body:\n{}", String::from_utf8_lossy(&buffer[body_offset..]));
// }

struct ProxyServer {
    config: Config,
}

impl ProxyServer {
    fn new(config_path: &str) -> Self {
        ProxyServer {
            config: Config::new(config_path),
        }
    }
    // WIP: Make location_handler

    async fn location_handler(&self, location: &Location, stream: &mut TcpStream) {
        let client = HttpClient::new();

        let req = RpgxRequest {
            url: Url::parse(location.proxy_pass.as_str()).unwrap(),
            method: "GET".to_string(),
            headers: [
                ("Accept".to_string(), "*/*".to_string()),
                ("Host".to_string(), "localhost:3000".to_string()),
                ("Connection".to_string(), "close".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
            body: Vec::new(),
        };

        let res = client.execute(&req).await;

        if res.is_err() {
            let msg = res.err().unwrap();
            error!("[location_handler]: {}", msg);
            return;
        }

        res.unwrap().send(stream).await;
    }

    async fn request_handler(&self, stream: &mut TcpStream) {
        let mut buffer = vec![0; self.config.max_tcp_buffer_size];

        let buffer_size: usize = match stream.read(&mut buffer).await {
            Ok(buffer_size) => buffer_size,
            Err(error) => {
                eprintln!("Failed to read from stream; Error: {:?}", error);
                return;
            }
        };

        debug!("Incoming connection. Buffer size: {}", buffer_size);

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let body_offset_result = req.parse(&buffer[0..buffer_size]);

        if body_offset_result.is_err() {
            warn!(
                "Failed to parse HTTP request. Error: {}",
                body_offset_result.err().unwrap()
            );

            return;
        }

        let body_offset = body_offset_result.unwrap();

        if body_offset.is_partial() {
            warn!("Partial request is not supported");
            RpgxResponse::make_internal_error().send(stream).await;
            return;
        }

        let path = req.path.unwrap();

        debug!(
            "Incoming request HTTP/{} {} {}",
            req.version.unwrap(),
            req.method.unwrap(),
            path
        );

        let location_handler = self.config.location.get(path);

        if location_handler.is_none() {
            RpgxResponse::new(404).send(stream).await;
            return;
        }

        // let handler = location_handler.unwrap().proxy_pass;
        self.location_handler(&location_handler.unwrap(), stream)
            .await;

        // RpgxResponse::new(401).send(stream).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(
        env_logger::Env::default()
            .filter_or("RUST_LOG", "debug")
            .write_style_or("RUST_LOG_STYLE", "always"),
    );

    let proxy_server = ProxyServer::new("config.toml");

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let manager = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

        while let Ok(connection) = listener.accept().await {
            let (socket, _) = connection;

            tx.send(socket).await.unwrap();
        }
    });

    let handler = tokio::spawn(async move {
        while let Some(mut stream) = rx.recv().await {
            proxy_server.request_handler(&mut stream).await;
            stream.shutdown().await.unwrap();
            continue;

            // let mut buf = [0; 1024];

            // let n: usize = match stream.read(&mut buf).await {
            //     Ok(n) => n,
            //     Err(error) => {
            //         eprintln!("Failed to read from stream; Error: {:?}", error);
            //         RpgxResponse::make_internal_error().send(&mut stream).await;
            //         continue;
            //     }
            // };

            // let mut headers = [httparse::EMPTY_HEADER; 64];
            // let mut req = httparse::Request::new(&mut headers);
            // let data_buffer = &buf[0..n];

            // let body_offset = req.parse(data_buffer).unwrap().unwrap();

            // println!("Method {}", req.method.unwrap());
            // println!("Version {}", req.version.unwrap_or(0));
            // println!("Path {}", req.path.unwrap());

            // let http_header = format!("{} {} HTTP/1.1", req.method.unwrap(), req.path.unwrap());
            // let headers = req
            //     .headers
            //     .to_vec()
            //     .into_iter()
            //     .map(|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap()))
            //     .collect();

            // let req_str = vec![http_header, headers, "\r\n".to_string()].join("\r\n");

            // println!("HTTP Request String:\n{}", req_str);

            // let res_val = TcpStream::connect("127.0.0.1:3000").await;

            // if res_val.is_err() {
            //     error!(
            //         "Failed to connect to :3000, Error: {}",
            //         res_val.err().unwrap()
            //     );
            //     RpgxResponse::make_internal_error().send(&mut stream).await;
            //     return;
            // }

            // let mut req_stream: TcpStream = res_val.unwrap();

            // req_stream.write_all(req_str.as_bytes()).await.unwrap();

            // // let mut buffer = Vec::new();
            // // let recv_size = stream.read_to_end(&mut buffer).await.unwrap();

            // let mut buffer_raw = [0; 1024 * 8];
            // let recv_size = req_stream.read(&mut buffer_raw).await.unwrap();
            // let buffer = &buffer_raw[0..recv_size];

            // println!("Reveive {} bytes", recv_size);

            // let mut headers_slice = [httparse::EMPTY_HEADER; 64];
            // let mut response = httparse::Response::new(&mut headers_slice);
            // let body_offset = response.parse(&buffer).unwrap().unwrap();
            // let body = &buffer[body_offset..];

            // let result_http_header = format!(
            //     "HTTP/1.1 {} {}",
            //     response.code.unwrap(),
            //     response.reason.unwrap(),
            // );

            // let result_headers: String = response
            //     .headers
            //     .to_vec()
            //     .into_iter()
            //     .map(|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap()))
            //     .collect();

            // let result_str = vec![
            //     result_http_header,
            //     result_headers,
            //     String::from_utf8_lossy(body).to_string(),
            // ]
            // .join("\r\n");

            // let response_opt = stream.write_all(result_str.as_bytes()).await;

            // if response_opt.is_err() {
            //     panic!(
            //         "Failed to send data to receiver, Error: {}",
            //         response_opt.err().unwrap()
            //     );
            // }

            // response_opt.unwrap();

            // // let request = http::Request::builder()
            // //     .uri("127.0.0.1:3000")
            // //     .body(())
            // //     .unwrap();

            // // connection
            // //     .write_all(b"GET / HTTP/1.1\r\nHost: localhost:3000")
            // //     .await
            // //     .unwrap();
            // // connection.write_all(data_buffer).await.unwrap();
            // // connection.peek(&mut b1).await.unwrap();

            // // make_request(&req).await;

            // // println!("DATA {}", response.reason.unwrap());

            // // stream.write_all(&b1);
        }
    });

    manager.await.unwrap();
    handler.await.unwrap();
    Ok(())
}

// async fn handler() -> Result<()> {
//
// }
