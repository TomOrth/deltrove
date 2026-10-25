use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("TARGET_READY");
    io::stdout().flush()?;
    for line in io::stdin().lock().lines() {
        match line?.as_str() {
            "ping" => println!("PONG"),
            "exit" => break,
            "work" => println!("WORK_DONE"),
            other => println!("UNKNOWN:{other}"),
        }
        io::stdout().flush()?;
    }
    Ok(())
}
