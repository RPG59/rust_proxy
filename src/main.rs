use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    net::TcpStream
};

use http;

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

            let n = match stream.read(&mut buf).await {
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

            for header in req.headers {
                println!("Header {} : {}", header.name, String::from_utf8_lossy(header.value));
            }

            println!("Raw {}", String::from_utf8_lossy(data_buffer));
            println!("Body {}", String::from_utf8_lossy(&buf[body_offset..n]));

            let request = http::Request::builder().uri("127.0.0.1:3000").body(()).unwrap();

            let mut connection = TcpStream::connect("127.0.0.1:3000").await.unwrap();
            connection.write_all(b"GET / HTTP/1.1\r\nHost: localhost:3000").await.unwrap();
            // connection.write_all(data_buffer).await.unwrap();
            let mut b1 = [0; 1024];
            connection.peek(&mut b1).await.unwrap();

            println!("DATA {}", String::from_utf8_lossy(&b1));

            stream.write_all(&b1);
        }
    });

    manager.await.unwrap();
    handler.await.unwrap();
    Ok(())
}

// async fn handler() -> Result<()> {
//
// }



