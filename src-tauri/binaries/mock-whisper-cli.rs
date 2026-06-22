use std::env;
use std::io::{self, Read};
use std::thread;
use std::time::Duration;

fn main() {
    // Parse arguments roughly
    let args: Vec<String> = env::args().collect();
    
    // Check if we need to read from stdin or file parameters
    let mut read_stdin = false;
    let mut file_path = None;
    let mut output_txt = false;
    
    for (i, arg) in args.iter().enumerate() {
        if arg == "-f" || arg == "--file" {
            if i + 1 < args.len() {
                if args[i + 1] == "-" {
                    read_stdin = true;
                } else {
                    file_path = Some(&args[i + 1]);
                }
            }
        }
        if arg == "--output-txt" || arg == "-otxt" {
            output_txt = true;
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
        
    if output_txt {
        if let Some(path) = file_path {
            let out_path = format!("{}.txt", path);
            let _ = std::fs::write(&out_path, &transcription);
        }
    }
    
    // Print transcription to stdout
    println!("{}", transcription);
}
