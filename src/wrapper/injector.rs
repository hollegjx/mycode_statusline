use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub struct ClaudeCodeInjector {
    claude_path: PathBuf,
    translation_enabled: bool,
}

impl ClaudeCodeInjector {
    pub fn new(
        claude_path: PathBuf,
        _translation_config: Option<()>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            claude_path,
            translation_enabled: false,
        })
    }

    pub fn start(&self, args: Vec<String>) -> Result<Child, Box<dyn std::error::Error>> {
        let mut cmd = if cfg!(target_os = "windows")
            && self.claude_path.extension().is_some_and(|ext| ext == "cmd")
        {
            // On Windows, .cmd files need to be run through cmd.exe
            let mut c = Command::new("cmd");
            c.arg("/C");
            c.arg(&self.claude_path);
            c
        } else {
            Command::new(&self.claude_path)
        };

        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variable to indicate wrapper is active
        cmd.env("UUCODE_WRAPPER", "1");
        cmd.env("UUCODE_VERSION", env!("CARGO_PKG_VERSION"));

        let child = cmd.spawn()?;
        Ok(child)
    }

    pub fn intercept_input(&self, input: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(input.to_string())
    }

    pub fn intercept_output(&self, output: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(output.to_string())
    }

    pub fn run_with_interception(
        &mut self,
        args: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 默认情况下直接运行 Claude Code
        if !self.translation_enabled {
            let mut cmd = if cfg!(target_os = "windows")
                && self.claude_path.extension().is_some_and(|ext| ext == "cmd")
            {
                let mut c = Command::new("cmd");
                c.arg("/C");
                c.arg(&self.claude_path);
                c
            } else {
                Command::new(&self.claude_path)
            };

            cmd.args(&args);

            // Set environment variable to indicate wrapper is active
            cmd.env("UUCODE_WRAPPER", "1");
            cmd.env("UUCODE_VERSION", env!("CARGO_PKG_VERSION"));

            // Inherit stdin/stdout/stderr for interactive use
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let status = cmd.status()?;

            if !status.success() {
                return Err(format!("Claude Code exited with status: {}", status).into());
            }

            return Ok(());
        }

        // Translation enabled - intercept I/O
        let mut child = self.start(args)?;

        let stdin = child.stdin.take().ok_or("Failed to capture stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        // Spawn thread to handle stdout
        let stdout_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("{}", line);
            }
        });

        // Spawn thread to handle stderr
        let stderr_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("{}", line);
            }
        });

        // Handle stdin
        let stdin_handle = std::thread::spawn(move || {
            let mut stdin_writer = stdin;
            let stdin_reader = std::io::stdin();
            for line in stdin_reader.lock().lines().map_while(Result::ok) {
                if let Err(e) = writeln!(stdin_writer, "{}", line) {
                    eprintln!("Error writing to Claude Code stdin: {}", e);
                    break;
                }
            }
        });

        // Wait for child process
        let status = child.wait()?;

        // Wait for threads
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();
        let _ = stdin_handle.join();

        if !status.success() {
            return Err(format!("Claude Code exited with status: {}", status).into());
        }

        Ok(())
    }
}
