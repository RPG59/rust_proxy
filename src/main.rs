use std::fmt::{format, Pointer};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    net::TcpStream,
};

use http;
use httparse;
use reqwest;

/*
   TODO:
       - Config Lang
           -- server_name
           -- location
           -- proxy_set_header
           -- add_header
           -- proxy_pass

*/

async fn make_request<'i>(req: &httparse::Request<'i, 'i>) {
    let http_header = format!("{} {} HTTP/1.1", req.method.unwrap(), req.path.unwrap());
    let headers = req
        .headers
        .to_vec()
        .into_iter()
        .map((|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap())))
        .collect();

    let req_str = vec![http_header, headers, "\r\n".to_string()].join("\r\n");

    println!("HTTP Request String:\n{}", req_str);

    let mut stream: TcpStream = TcpStream::connect("127.0.0.1:3000").await.unwrap();

    stream.write_all(req_str.as_bytes()).await.unwrap();

    // let mut buffer = Vec::new();
    // let recv_size = stream.read_to_end(&mut buffer).await.unwrap();
    let mut buffer_raw = [0; 1024 * 8];
    let recv_size = stream.read(&mut buffer_raw).await.unwrap();
    let buffer = &buffer_raw[0..recv_size];

    println!("Reveive {} bytes", recv_size);

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Response::new(&mut headers);
    let body_offset = req.parse(&buffer).unwrap().unwrap();

    println!(
        "Headers:\n{}",
        String::from_utf8_lossy(&buffer[0..body_offset])
    );
    println!("Body:\n{}", String::from_utf8_lossy(&buffer[body_offset..]));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            let mut buf = [0; 1024];

            let n: usize = match stream.read(&mut buf).await {
                Ok(n) => n,
                Err(error) => {
                    eprintln!("Failed to read from stream; Error: {:?}", error);
                    return;
                }
            };

            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);
            let data_buffer = &buf[0..n];

            let body_offset = req.parse(data_buffer).unwrap().unwrap();

            // println!("Read data: {:?}", String::from_utf8_lossy(&buf[0..n]));
            println!("Method {}", req.method.unwrap());
            println!("Version {}", req.version.unwrap_or(0));
            println!("Path {}", req.path.unwrap());

            // for header in req.headers {
            //     println!(
            //         "Header {} : {}",
            //         header.name,
            //         String::from_utf8_lossy(header.value)
            //     );
            // }

            // println!("Raw {}", String::from_utf8_lossy(data_buffer));
            // println!("Body {}", String::from_utf8_lossy(&buf[body_offset..n]));

            let res_body = make_request(&req).await;

            // let request = http::Request::builder()
            //     .uri("127.0.0.1:3000")
            //     .body(())
            //     .unwrap();

            // connection
            //     .write_all(b"GET / HTTP/1.1\r\nHost: localhost:3000")
            //     .await
            //     .unwrap();
            // connection.write_all(data_buffer).await.unwrap();
            // connection.peek(&mut b1).await.unwrap();

            // make_request(&req).await;

            // println!("DATA {}", String::from_utf8_lossy(&b1));

            // stream.write_all(&b1);
        }
    });

    manager.await.unwrap();
    handler.await.unwrap();
    Ok(())
}

// async fn handler() -> Result<()> {
//
// }
