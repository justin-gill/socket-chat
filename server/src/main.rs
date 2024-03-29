use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::env;
use std::process;

use sha2::{Digest, Sha256};

use common::{MESSAGE_SIZE, LOCAL_HOST, validate_port_arg};

const DEFAULT_USERNAME_LENGTH: usize = 10;

struct Client {
    username: String,
    stream: TcpStream,
}

#[derive(Clone)]
struct Message {
    content: String,
    identifier: String,
}

/// Handles the initial connection and spawning threads
fn main() {
    let (port, username_length) = parse_args();

    let listener = setup_listener(port);
    
    let mut clients: Vec<Client> = Vec::new();
    // this channel will be used for sending messages between threads
    let (sender, receiver) = mpsc::channel::<Message>();

    loop {   
        if let Ok((socket, address)) = listener.accept() {
            println!("Accepted {}", address);
            // clone the sender for the thread
            let sender = sender.clone();

            let username = generate_username(&address, username_length);
            clients.push(Client { username, stream:socket.try_clone().expect("Failed to clone client")});
            
            thread::spawn(move || {
                message_loop(socket, sender);
            });
        }
        if let Ok(message) = receiver.try_recv() 
        {
            println!("Received message");
            clients = send_message_to_clients(clients, message);
        }
    }
}            

/// Parses server args ( port and username length )
fn parse_args() -> (String, usize) {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        2 => {
            let port = validate_port_arg(&args[1]);
            (port, DEFAULT_USERNAME_LENGTH)
        },
        3 => {
            let port = validate_port_arg(&args[1]);
            let username_length = args[2].parse::<usize>().unwrap_or_else(|_| {
                eprintln!("Username length must be an integer.");
                process::exit(1);
            });
            (port, username_length)
        },
        _ => {
            eprintln!("Usage: cargo run --bin server PORT [USERNAME_LENGTH]");
            eprintln!("Example: cargo run --bin server 8080 32");
            process::exit(1);
        },
    }
}

/// Sets up the TCP Listener given the port
fn setup_listener(port: String) -> TcpListener {
    let socket_address = format!("{}:{}", LOCAL_HOST, port);
    let listener = TcpListener::bind(socket_address).expect("Could not bind socket");
    listener.set_nonblocking(true).expect("Failed to set non-blocking");
    listener
}

/// Listens for new messages from clients and sends to channel
fn message_loop(mut socket: TcpStream, sender: mpsc::Sender<Message>) {
    loop {
        let mut buffer = vec![0; MESSAGE_SIZE];
        match socket.read(&mut buffer) {
            Ok(0) => {
                println!("Connection closed");
                break;
            }
            Ok(_) => {
                let message = match String::from_utf8(buffer) {
                    Ok(msg) => msg.trim_matches(char::from(0)).to_string(),
                    Err(e) => {
                        println!("Failed to parse message as UTF-8: {}", e);
                        return;
                    },
                };
                sender.send(Message {content: message, identifier: socket.peer_addr().unwrap().to_string()}).expect("Failed to send message to receiver");
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("Closing connection");
                break;
            }
        }
    }
}

/// Generates username using SHA256 hash from the Clients socket address
fn generate_username(address: &std::net::SocketAddr, length: usize) -> String {
    let port = address.port();
    let port_bytes = port.to_be_bytes();
    let mut hasher = Sha256::new();
    hasher.update(&port_bytes);
    let hash_result = hasher.finalize();
    let hex_string = format!("{:x}", hash_result);
    let truncated_hex = &hex_string[..length];
    truncated_hex.to_string()
}

/// Sends messages to clients and maintains active user's list
fn send_message_to_clients(clients: Vec<Client>, message: Message) -> Vec<Client> {
    clients
        .into_iter()
        .filter_map(|mut client| {
            // Check if the client's identifier is different from the message's identifier
            if client.stream.peer_addr().ok() != Some(message.identifier.parse().unwrap()) {
                let full_message = format!("{}: '{}'", client.username, message.content);
                let mut buffer = full_message.into_bytes();
                buffer.resize(MESSAGE_SIZE, 0);
                println!("Sending message to user {}", client.username);
                if let Err(_) = client.stream.write_all(&buffer) {
                    // If writing fails, the connection is closed and we should remove the client
                    println!("Failed to send message to {}", client.username);
                    return None;
                }
            }
            Some(client)
        })
        .collect::<Vec<_>>()
}


