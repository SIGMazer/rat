use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::spawn;

struct Client {
    conn: Arc<TcpStream>,
    name: String,
}

enum Message {
    CONNECT { conn: Arc<TcpStream>, name: String },
    DISCONNECT { conn: SocketAddr },
    NEW { conn: SocketAddr, content: String },
}
fn server(receiver: Receiver<Message>) {
    let mut clients: HashMap<SocketAddr, Arc<Client>> = HashMap::new();
    loop {
        match receiver.recv() {
            Ok(message) => match message {
                Message::CONNECT { conn: client, name } => {
                    let client = Arc::new(Client { conn: client, name });
                    clients.insert(client.conn.peer_addr().unwrap(), client.clone());
                    client
                        .conn
                        .deref()
                        .write(b"Welcome to the chat!\n")
                        .map_err(|e| eprintln!("Failed to send: {}", e))
                        .unwrap();

                    println!("{}, connected", client.name);
                }
                Message::DISCONNECT { conn: client } => {
                    let client = clients.remove(&client).unwrap();
                    client
                        .conn
                        .deref()
                        .shutdown(std::net::Shutdown::Both)
                        .map_err(|e| eprintln!("Failed to shutdown: {}", e))
                        .unwrap();
                    println!("{}, disconnected", client.name);
                }
                Message::NEW {
                    content,
                    conn: client,
                } => {
                    println!("{}: {}", client, content);
                    if content.eq("\n") {
                        continue;
                    }
                    let client = clients.get(&client).unwrap();
                    let conn = client.conn.peer_addr().unwrap();
                    for (addr, c) in clients.iter() {
                        if addr.eq(&conn) {
                            continue;
                        }
                        c.conn
                            .deref()
                            .write(format!("{}: {}", client.name, content).as_bytes())
                            .map_err(|e| eprintln!("Failed to send: {}", e))
                            .unwrap();
                    }
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn client(client: Arc<TcpStream>, sender: Sender<Message>) {
    let mut name = [0; 1024];
    client
        .deref()
        .write(b"Enter your name: ")
        .map_err(|e| eprintln!("Failed to send: {}", e))
        .unwrap();
    client
        .deref()
        .read(&mut name)
        .map_err(|e| eprintln!("Failed to read: {}", e))
        .unwrap();

    let name = String::from_utf8_lossy(&name).to_string().replace("\n", "");

    sender
        .send(Message::CONNECT {
            conn: client.clone(),
            name,
        })
        .map_err(|e| eprintln!("Failed to connect to the server: {}", e))
        .unwrap();

    let client_addr = client
        .peer_addr()
        .map_err(|e| eprintln!("Failed to get client address: {}", e))
        .unwrap();
    loop {
        let mut buffer = [0; 1024];
        match client.deref().read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    sender
                        .send(Message::DISCONNECT { conn: client_addr })
                        .map_err(|e| eprintln!("Failed to send: {}", e))
                        .unwrap();
                    break;
                }
                let content = String::from_utf8_lossy(&buffer[..n]).to_string();
                sender
                    .send(Message::NEW {
                        content,
                        conn: client_addr,
                    })
                    .map_err(|e| eprintln!("Failed to send: {}", e))
                    .unwrap();
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:1337")
        .map_err(|e| eprintln!("Failed to bind: {}", e))
        .unwrap();
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
