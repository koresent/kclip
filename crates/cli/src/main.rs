mod daemon;

use std::path::PathBuf;
use std::time::Duration;
use clap::{Parser, Subcommand};
use kclip_config::AppConfig;
use kclip_ipc::{resolve_socket_path, IpcClient, IpcCommand, IpcResponse};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "kclip", version = "0.1.0", about = "High-performance minimal Wayland clipboard manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, help = "Path to custom YAML configuration file")]
    config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Start the background clipboard daemon")]
    Daemon,

    #[command(about = "Toggle the floating clipboard window (default action)")]
    Toggle,

    #[command(about = "Show the floating clipboard window")]
    Show,

    #[command(about = "Hide the floating clipboard window")]
    Hide,

    #[command(about = "Query daemon and database status")]
    Status,

    #[command(about = "Restore an item from history to the clipboard")]
    Paste {
        #[arg(help = "Database ID of the item to paste")]
        id: i64,
    },

    #[command(about = "Gracefully stop the running daemon")]
    Quit,
}

fn ui_pid_file_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join("kclip-ui.pid")
}

fn get_running_ui_pid(pid_file: &std::path::Path) -> Option<i32> {
    if !pid_file.exists() {
        return None;
    }
    if let Ok(content) = std::fs::read_to_string(pid_file) {
        if let Ok(pid) = content.trim().parse::<i32>() {
            if unsafe { libc::kill(pid, 0) } == 0 {
                if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                    if cmdline.contains("kclip") {
                        return Some(pid);
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_file(pid_file);
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let config = if let Some(path) = &cli.config {
        AppConfig::load_from_file(path)?
    } else if let Some(default_path) = AppConfig::default_config_path() {
        if default_path.exists() {
            AppConfig::load_from_file(&default_path)?
        } else {
            AppConfig::default()
        }
    } else {
        AppConfig::default()
    };

    let log_level = match config.logging.level.to_ascii_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let socket_path = resolve_socket_path(config.app.socket_path.as_ref());

    match cli.command.unwrap_or(Commands::Toggle) {
        Commands::Daemon => {
            daemon::run_daemon(config)?;
        }
        Commands::Toggle => {
            let pid_file = ui_pid_file_path();
            if let Some(pid) = get_running_ui_pid(&pid_file) {
                let _ = unsafe { libc::kill(pid, libc::SIGTERM) };
                let _ = std::fs::remove_file(&pid_file);
                return Ok(());
            }

            let current_pid = std::process::id();
            let _ = std::fs::write(&pid_file, current_pid.to_string());
            spawn_wayland_focus_warp(current_pid);
            let res = kclip_ui::launch_ui(config);
            let _ = std::fs::remove_file(&pid_file);
            res?;
        }
        Commands::Show => {
            let pid_file = ui_pid_file_path();
            if get_running_ui_pid(&pid_file).is_some() {
                return Ok(());
            }

            let current_pid = std::process::id();
            let _ = std::fs::write(&pid_file, current_pid.to_string());
            spawn_wayland_focus_warp(current_pid);
            let res = kclip_ui::launch_ui(config);
            let _ = std::fs::remove_file(&pid_file);
            res?;
        }
        Commands::Hide => {
            let pid_file = ui_pid_file_path();
            if let Some(pid) = get_running_ui_pid(&pid_file) {
                let _ = unsafe { libc::kill(pid, libc::SIGTERM) };
                let _ = std::fs::remove_file(&pid_file);
            }
        }
        Commands::Status => {
            send_or_report(&socket_path, IpcCommand::Status);
        }
        Commands::Paste { id } => {
            send_or_report(&socket_path, IpcCommand::Paste { id });
        }
        Commands::Quit => {
            send_or_report(&socket_path, IpcCommand::Quit);
        }
    }

    Ok(())
}

fn send_or_report(socket_path: &PathBuf, cmd: IpcCommand) {
    match IpcClient::send(socket_path, &cmd, Duration::from_millis(1000)) {
        Ok(IpcResponse::Ok { message }) => {
            println!("{}", message);
        }
        Ok(IpcResponse::Status { running, item_count }) => {
            println!("kclip daemon: running={}", running);
            println!("Total history items: {}", item_count);
        }
        Ok(IpcResponse::Error { message }) => {
            eprintln!("Error from daemon: {}", message);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Could not communicate with kclip daemon: {}", e);
            eprintln!("Make sure 'kclip daemon' is running.");
            std::process::exit(1);
        }
    }
}

fn spawn_wayland_focus_warp(pid: u32) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(40));
        let uid = unsafe { libc::getuid() };
        let sock_path = format!("/run/user/{uid}/umbriel-wayland-0.sock");
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(sock_path) {
            use std::io::{BufRead, BufReader, Write};
            let _ = stream.write_all(b"{\"cmd\":\"windows\"}\n");
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(wins) = val.get("ok").and_then(|v| v.as_array()) {
                        for w in wins {
                            if w.get("pid").and_then(|p| p.as_u64()) == Some(pid as u64) {
                                if let Some(id) = w.get("id").and_then(|i| i.as_str()) {
                                    let _ = std::process::Command::new("umbriel")
                                        .args(["msg", "window-focus-warp", id])
                                        .status();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}
