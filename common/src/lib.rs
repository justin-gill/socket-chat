use std::process;

pub const MESSAGE_SIZE: usize = 256;
pub const LOCAL_HOST: &str = "127.0.0.1";

// Ensures port is valid number, returns string
pub fn validate_port_arg(port_arg: &str) -> String {
    // u16 handles port max (65535)
    match port_arg.parse::<u16>() {
        Ok(_) => port_arg.to_string(),
        Err(_) => {
            eprintln!("Port must be a valid number between 0 and 65535.");
            process::exit(1);
        }
    }
}

