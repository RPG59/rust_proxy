use std::fmt::{format, Pointer};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
};

use http;
use httparse::{self, Header};
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

struct Response<'a> {
    headers: Vec<httparse::Header<'a>>,
    body: Vec<u8>,
}

async fn make_request<'a>(req: &httparse::Request<'a, 'a>) -> Box<Response<'a>> {
    let http_header = format!("{} {} HTTP/1.1", req.method.unwrap(), req.path.unwrap());
    let headers = req
        .headers
        .to_vec()
        .into_iter()
        .map(|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap()))
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

    // let mut headers = Box::new([httparse::EMPTY_HEADER; 64]);
    let mut headers_slice = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Response::new(&mut headers_slice);
    let body_offset = req.parse(&buffer).unwrap().unwrap();

    // for header in headers {
    //     println!(
    //         "Header: {}:{}",
    //         header.name,
    //         String::from_utf8_lossy(header.value)
    //     )
    // }

    // println!(
    //     "Headers:\n{}",
    //     String::from_utf8_lossy(&buffer[0..body_offset])
    // );

    let mut body = Vec::with_capacity(recv_size - body_offset);
    body.clone_from_slice(&buffer[body_offset..]);

    ////aaaaaaaaaaaaaaaaaa
    let mut headers = Vec::with_capacity(req.headers.len());

    for header in req.headers {
        let test = header.clone();

        headers.push(test);
    }
    ///////

    Box::new(Response {
        headers: vec![],
        body,
    })

    // println!("Body:\n{}", String::from_utf8_lossy(&buffer[body_offset..]));
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

            let http_header = format!("{} {} HTTP/1.1", req.method.unwrap(), req.path.unwrap());
            let headers = req
                .headers
                .to_vec()
                .into_iter()
                .map(|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap()))
                .collect();

            let req_str = vec![http_header, headers, "\r\n".to_string()].join("\r\n");

            println!("HTTP Request String:\n{}", req_str);

            let res_val = TcpStream::connect("127.0.0.1:3000").await;

            if res_val.is_err() {
                let error = res_val.err().unwrap();

                panic!("Failed to connect to :3000, Error: {}", error);
            }

            let mut req_stream: TcpStream = res_val.unwrap();

            req_stream.write_all(req_str.as_bytes()).await.unwrap();

            // let mut buffer = Vec::new();
            // let recv_size = stream.read_to_end(&mut buffer).await.unwrap();

            let mut buffer_raw = [0; 1024 * 8];
            let recv_size = req_stream.read(&mut buffer_raw).await.unwrap();
            let buffer = &buffer_raw[0..recv_size];

            println!("Reveive {} bytes", recv_size);

            let mut headers_slice = [httparse::EMPTY_HEADER; 64];
            let mut response = httparse::Response::new(&mut headers_slice);
            let body_offset = response.parse(&buffer).unwrap().unwrap();
            let body = &buffer[body_offset..];

            let result_http_header = format!(
                "HTTP/1.1 {} {}",
                response.code.unwrap(),
                response.reason.unwrap(),
            );

            let result_headers: String = response
                .headers
                .to_vec()
                .into_iter()
                .map(|x| format!("{}:{}\r\n", x.name, std::str::from_utf8(x.value).unwrap()))
                .collect();

            let result_str = vec![
                result_http_header,
                result_headers,
                String::from_utf8_lossy(body).to_string(),
            ]
            .join("\r\n");

            let response_opt = stream.write_all(result_str.as_bytes()).await;

            if response_opt.is_err() {
                panic!(
                    "Failed to send data to receiver, Error: {}",
                    response_opt.err().unwrap()
                );
            }

            response_opt.unwrap();

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

            // println!("DATA {}", response.reason.unwrap());

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
