use std::env;
use std::io::{self, Read};
use std::thread;
use std::time::Duration;

fn main() {
    // Parse arguments roughly
    let args: Vec<String> = env::args().collect();
    
    // Check if we need to read from stdin
    let mut read_stdin = false;
    for (i, arg) in args.iter().enumerate() {
        if arg == "-f" && i + 1 < args.len() && args[i + 1] == "-" {
            read_stdin = true;
        }
    }
    
    if read_stdin {
        // Read everything from stdin to satisfy the process write
        let mut buffer = Vec::new();
        let mut stdin = io::stdin();
        let _ = stdin.read_to_end(&mut buffer);
    }
    
    // Simulate transcription latency
    thread::sleep(Duration::from_millis(300));
    
    // Get the mock transcription from environment variable
    let transcription = env::var("KOTOBA_MOCK_TRANSCRIPTION")
        .unwrap_or_else(|_| "mock transcription".to_string());
        
    // Print transcription to stdout
    println!("{}", transcription);
}
