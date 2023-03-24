use clap::Parser;
use nix::sched::unshare;
use std::ffi::CString;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Unshare IPC Namespaces
    #[arg(long, default_value_t = false)]
    ipc: bool,
    /// Unshare Mount Namespace
    #[arg(long, default_value_t = false)]
    mount: bool,
    /// Unshare Network Namespace
    #[arg(long, default_value_t = false)]
    network: bool,
    /// Unshare PID Namespace
    #[arg(long, default_value_t = false)]
    pid: bool,
    /// Unshare UTS Namespace
    #[arg(long, default_value_t = false)]
    uts: bool,
    /// Unshare UserNS Namespace
    #[arg(long, default_value_t = false)]
    userns: bool,
    command: String
}

fn main() -> std::process::ExitCode {
    let mut flags: nix::sched::CloneFlags = nix::sched::CloneFlags::empty();
    let _args = Args::parse();
    if _args.ipc {
        flags |= nix::sched::CloneFlags::CLONE_NEWIPC
    }
    if _args.mount {
        flags |= nix::sched::CloneFlags::CLONE_NEWNS
    }
    if _args.network {
        flags |= nix::sched::CloneFlags::CLONE_NEWNET
    }
    if _args.pid {
        flags |= nix::sched::CloneFlags::CLONE_NEWPID
    }
    if _args.uts {
        flags |= nix::sched::CloneFlags::CLONE_NEWUTS
    }
    if _args.userns {
        flags |= nix::sched::CloneFlags::CLONE_NEWUSER
    }
    if _args.command.is_empty() {
        println!("invalid command");
        return std::process::ExitCode::from(1)
    }

    // Try Unshare with given flags
    if let Err(err) = unshare(flags) {
        println!("got error while unshare: {}", err);
        return std::process::ExitCode::from(2)
    }

    let command = CString::new(_args.command.clone()).unwrap();
    let args = [CString::new(_args.command.clone()).unwrap()];

    if let Err(err) = nix::unistd::execvp(&command, &args) {
        println!("got error while execvp: {}", err);
        return std::process::ExitCode::from(2)
    }

    return std::process::ExitCode::SUCCESS
}
