use std::io::{self, ErrorKind, Read, Write};
use std::sync::mpsc::{self, TryRecvError};
use std::net::TcpStream;
use std::thread;
use std::env;
use std::process;

use common::{MESSAGE_SIZE, LOCAL_HOST, validate_port_arg};

/// Handles initial connection, spawning threads, and invoke main loop
fn main() {
    let port = parse_args();
    let client = setup_stream(port);
    let (sender, receiver) = mpsc::channel::<String>();
    thread::spawn(move || listen_to_server(client, receiver));
    send_messages(sender);
}

/// Parses the client args ( Port )
fn parse_args() -> String {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        2 => {
            let port = validate_port_arg(&args[1]);
            port
        },
        _ => {
            eprintln!("Usage: cargo run --bin client PORT");
            eprintln!("Example: cargo run --bin client 8080");
            process::exit(1);
        },
    }
}

/// Sets up TCP Stream given the port
fn setup_stream(port: String) -> TcpStream {
    let socket_address = format!("{}:{}", LOCAL_HOST, port);
    let stream = TcpStream::connect(socket_address).expect("Failed to connect");
    stream.set_nonblocking(true).expect("Failed to initiate non-blocking");
    stream
}

/// Listens to the server and displays messages
fn listen_to_server(mut client: TcpStream, receiver: mpsc::Receiver<String>) {
    loop {
        let mut buffer = vec![0; MESSAGE_SIZE];
        match client.read_exact(&mut buffer) {
            Ok(_) => {
                let trimmed_buffer = buffer.iter().cloned().take_while(|&x| x != 0).collect::<Vec<_>>();
                let message = match String::from_utf8(trimmed_buffer) {
                    Ok(msg) => msg.trim_matches(char::from(0)).to_string(),
                    Err(e) => {
                        println!("Failed to parse message as UTF-8: {}", e);
                        return;
                    }
                };
                print!("\n{:}\n> ", message);
                io::stdout().flush().unwrap();
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("Connection with server was severed");
                break;
            }
        }
        match receiver.try_recv() {
            Ok(message) => {
                let mut buffer = message.clone().into_bytes();
                buffer.resize(MESSAGE_SIZE, 0);
                client.write_all(&buffer).expect("Writing to socket failed");
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }
    }
}

/// Listens for user input and sends messages to server
fn send_messages(sender: mpsc::Sender<String>) {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).expect("Reading from stdin failed");
        let message = buffer.trim().to_string();
        if message == "exit" || sender.send(message).is_err() {
            break;
        }
    }
}

