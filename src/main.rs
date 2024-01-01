use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write};
use std::ops::Deref;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::thread::spawn;
use std::sync::Arc;



struct Client {
    stream: Arc<TcpStream>,
}


enum Message {
    CONNECT{
        client: Arc<TcpStream>,
    },
    DISCONNECT{
        client: SocketAddr,
    },
    NEW{
        client: SocketAddr,
        content: String,
    },
}
fn server(receiver: Receiver<Message>) {
    let mut clients: Vec<Arc<Client>> = Vec::new();
    loop {
        match receiver.recv() {
            Ok(message) => {
                match message {
                    Message::CONNECT{client} => {
                        clients.push(Arc::new(Client{stream: client.clone()}));
                        println!("{}, connected", client.peer_addr().unwrap());
                    }
                    Message::DISCONNECT{client} => {
                        clients.retain(|c| c.stream.peer_addr().unwrap() != client);
                        println!("{}, disconnected", client);
                    }
                    Message::NEW{content, client} => {
                        println!("{}: {}", client, content);
                        if content.eq("\n") {
                            continue;
                        }
                        for c in clients.iter() {
                            if c.stream.peer_addr().unwrap() != client {
                                c.stream.deref().write(content.as_bytes())
                                    .map_err(|e| eprintln!("Failed to send: {}", e)).unwrap();
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn client(client: Arc<TcpStream>, sender: Sender<Message>) {
    sender.send(Message::CONNECT{client: client.clone()})
        .map_err(|e| eprintln!("Failed to send: {}", e)).unwrap();
    let client_addr = client.peer_addr()
        .map_err(|e| eprintln!("Failed to get client address: {}", e)).unwrap();
    loop {
        let mut buffer = [0; 1024];
        match client.deref().read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    sender.send(Message::DISCONNECT{client: client_addr })
                        .map_err(|e| eprintln!("Failed to send: {}", e)).unwrap();
                    break;
                }
                let content = String::from_utf8_lossy(&buffer[..n]).to_string();
                sender.send(Message::NEW{content, client: client_addr})
                    .map_err(|e| eprintln!("Failed to send: {}", e)).unwrap();
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

}



fn main() {
    let listener = TcpListener::bind("0.0.0.0:1337")
        .map_err(|e| eprintln!("Failed to bind: {}", e)).unwrap();
    println!("Listening on port 1337");

    let (sender, receiver) = channel::<Message>();
    spawn(|| server(receiver));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stream = Arc::new(stream);
                let sender = sender.clone();
                spawn(|| client(stream, sender));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }

    }
}
