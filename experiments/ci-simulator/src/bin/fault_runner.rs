use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Demo {
        #[arg(long)]
        target: String,
    },
}

/// Owns a direct child and its pipes; signals are sent only to this child PID.
struct OwnedTarget {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<std::io::Result<String>>,
    reaped: bool,
}

impl OwnedTarget {
    fn spawn(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or("child stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("child stdout unavailable")?;
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut owned = Self {
            child,
            stdin,
            lines,
            reaped: false,
        };
        owned.expect_line("TARGET_READY", Duration::from_secs(3))?;
        Ok(owned)
    }

    fn expect_line(
        &mut self,
        expected: &str,
        timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(format!("deadline waiting for {expected}").into());
            }
            let remaining = timeout.saturating_sub(start.elapsed());
            let line = self.lines.recv_timeout(remaining)?;
            if line?.trim() == expected {
                return Ok(());
            }
        }
    }

    fn command(&mut self, command: &str, response: &str) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()?;
        self.expect_line(response, Duration::from_secs(3))
    }

    #[cfg(unix)]
    fn pause(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: the PID comes from this owned Child and remains valid until this method returns.
        let result = unsafe { libc::kill(self.child.id() as i32, libc::SIGSTOP) };
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            let mut status = 0;
            // SAFETY: `status` is a live writable stack value and the PID is the owned child.
            let observed = unsafe {
                libc::waitpid(
                    self.child.id() as i32,
                    &mut status,
                    libc::WUNTRACED | libc::WNOHANG,
                )
            };
            if observed == self.child.id() as i32 && libc::WIFSTOPPED(status) {
                return Ok(());
            }
            if observed == self.child.id() as i32
                && (libc::WIFEXITED(status) || libc::WIFSIGNALED(status))
            {
                self.reaped = true;
                return Err("owned target exited before pause completed".into());
            }
            if observed < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if std::time::Instant::now() >= deadline {
                return Err("deadline waiting for owned child stop handshake".into());
            }
            thread::yield_now();
        }
    }

    #[cfg(unix)]
    fn resume(&self) -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: the PID comes from this owned Child and remains valid until this method returns.
        let result = unsafe { libc::kill(self.child.id() as i32, libc::SIGCONT) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().into())
        }
    }
}

impl Drop for OwnedTarget {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.reaped = true;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Commands::Demo { target } => {
            let mut child = OwnedTarget::spawn(&target)?;
            child.command("ping", "PONG")?;
            #[cfg(unix)]
            {
                child.pause()?;
                child.resume()?;
            }
            child.command("work", "WORK_DONE")?;
            child.child.kill()?;
            let status = child.child.wait()?;
            child.reaped = true;
            if status.success() {
                return Err("owned target unexpectedly exited cleanly after forced kill".into());
            }
            drop(child);
            let mut restarted = OwnedTarget::spawn(&target)?;
            restarted.command("ping", "PONG")?;
            writeln!(restarted.stdin, "exit")?;
            restarted.stdin.flush()?;
            let status = restarted.child.wait()?;
            restarted.reaped = true;
            if !status.success() {
                return Err(format!("restarted target exited with {status}").into());
            }
            println!(
                "observed owned-child ready, ping, pause/resume, work completion, kill, restart, ping and clean exit"
            );
        }
    }
    Ok(())
}
