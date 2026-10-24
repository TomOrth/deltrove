use std::{env, process};

fn main() {
    if let Err(error) = workload_fixtures::run(env::args_os().skip(1)) {
        eprintln!("error: {error}");
        process::exit(2);
    }
}
