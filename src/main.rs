use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

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

            let body_offset= req.parse(data_buffer).unwrap().unwrap();


            // println!("Read data: {:?}", String::from_utf8_lossy(&buf[0..n]));
            println!("Method {}", req.method.unwrap());
            println!("Version {}", req.version.unwrap_or(0));
            println!("Path {}", req.path.unwrap());

            for header in req.headers {
                println!("Header {} : {}", header.name, String::from_utf8_lossy(header.value));
            }

            println!("Body {}", String::from_utf8_lossy(&buf[body_offset..n]));
        }
    });

    manager.await.unwrap();
    handler.await.unwrap();
    Ok(())

    // loop {
    //     let (mut socket, _) = listener.accept().await?;
    //
    //     tokio::spawn(async move {
    //         let mut buf = [0; 1024];
    //
    //         loop {
    //             let n = match socket.read(&mut buf).await {
    //                 Ok(n) if n == 0 => return,
    //                 Ok(n) => n,
    //                 Err(e) => {
    //                     eprintln!("failed to read from socket; err = {:?}", e);
    //                     return;
    //                 }
    //             };
    //
    //             if let Err(e) = socket.write_all(&buf[0..n]).await {
    //                 eprintln!("failed to write to socket; err = {:?}", e);
    //                 return;
    //             }
    //         }
    //     });
    // }
}

// async fn handler() -> Result<()> {
//
// }




async fn get_connection() {
}


async fn send_data() {
    let connection = get_connection().await;

}


