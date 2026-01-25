use tokio::{
    net::TcpStream,
    io::{
        AsyncWriteExt,
    },
    sync::broadcast,
};
use crate::types::Frame;

pub async fn handle_client(
    mut stream: TcpStream,
    rx: &mut broadcast::Receiver<Frame>,
) {
    let addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("Could not get peer address. Exiting client handler.");
            return;
        }
    };
    println!("Handling client from address: {:?}", addr);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(mut frame) => {
                        println!("Sending frame to client: {:?}", frame);
                        match frame.to_pti_vector()
                        {
                            Ok(pty_frame) =>  {

                                let _hex_string: String = pty_frame.iter()
                                        .map(|byte| format!("{:02X}", byte))
                                        .collect::<Vec<String>>()
                                        .join(" ");

                                //println!("tx pti:{:?}", hex_string);
                                if let Err(e) = stream.write_all(&pty_frame).await {
                                    eprintln!("Client write failed: {e}");
                                    return;
                                }
                            }
                            Err(_e) => {
                                println!("Failed to form PTI packet");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("Client lagged: skipped {n} messages");
                    }
                    Err(_) => {
                        eprintln!("Some receive error happened.");
                    }
                }
            }


            // Detect client disconnect by trying to read
            result = stream.readable() => {
                if result.is_err() {
                    // Client dropped connection
                    eprintln!("Client disconnected (readable error): {:?}", addr);
                    return;
                }

                // Attempt a read of 0 bytes to check for EOF
                let mut buf = [0u8; 1];
                match stream.try_read(&mut buf) {
                    Ok(0) => {
                        // EOF => client disconnected
                        eprintln!("Client disconnected (EOF): {:?}", addr);
                        return;
                    }
                    Ok(msg) => {
                        // The client sent some data, but we don't care.
                        println!("Client sent {} bytes, ignoring", msg);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No real data; ignore
                        println!("No data from client");
                    }
                    Err(e) => {
                        eprintln!("Read error: {e}");
                        return;
                    }
                }
            }
        }
    }
}
