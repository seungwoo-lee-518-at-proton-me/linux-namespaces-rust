use std::{ffi::CString, vec};

use clap::Parser;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Execute Command in Child Process
    #[arg(short, long)]
    fork: bool,
    /// Join Specified Namespace
    #[arg(short, long)]
    namespace: Option<String>,
    /// Command
    command: Option<String>,
    /// Arguments
    args: Option<Vec<String>>
}

#[macro_use]
extern crate log;

impl std::fmt::Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ns: Option<String> = None;
        let mut args: Option<String> = None;
        let mut cmd: Option<String> = None;
        if let Some(namespace) = &self.namespace {
            ns = Some(namespace.to_string())
        }
        if let Some(argument) = &self.args {
            args = Some(argument.join(", ").to_string())
        }
        if let Some(command) = &self.command {
            cmd = Some(command.to_string())
        }
        write!(f, "<Arg fork={}, namespace={}, command={}, args={} />", 
               self.fork.to_string(), 
               ns.unwrap_or_default().to_string(),
               cmd.unwrap_or_default().to_string(), 
               args.unwrap_or_default().to_string())
    }
}

fn main() -> std::process::ExitCode {
    let mut cmd_args: Vec<CString> = vec![];
    env_logger::init();
    let args = Args::parse();
    if let Some(command_val) = &args.command {
        cmd_args.push(CString::new(command_val.as_str()).unwrap())
    }
    if let Some(args_val) = &args.args {
        for a in args_val {
            cmd_args.push(CString::new(a.as_str()).unwrap())
        }
    }
    if let Some(ns_dir) = &args.namespace {
        let namespace_fd = match nix::fcntl::open(ns_dir.as_str(), nix::fcntl::OFlag::O_RDONLY, nix::sys::stat::Mode::empty()) {
            Err(err) => {
                error!("open: {}", err);
                return std::process::ExitCode::from(1)
            },
            Ok(fd) => fd
        };
        if let Err(err) = nix::sched::setns(namespace_fd, nix::sched::CloneFlags::empty()) {
            error!("setns: {}", err);
            return std::process::ExitCode::from(3)
        }
    } else {
        error!("error, invalid --namespace (or -n) flag");
        return std::process::ExitCode::from(1)
    }
    if args.fork {
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                if let Err(err) = nix::sys::wait::waitpid(child, Some(nix::sys::wait::WaitPidFlag::empty())) {
                    error!("waitpid: {}", err);
                    return std::process::ExitCode::from(5)
                }
            },
            Ok(nix::unistd::ForkResult::Child) => {
                if let Err(err) = nix::unistd::execvp(cmd_args[0].as_ref(), &cmd_args) {
                    error!("execvp: {}", err);
                    return std::process::ExitCode::from(4)
                }
                return std::process::ExitCode::SUCCESS
            },
            Err(err) => {
                error!("fork: {}", err);
                return std::process::ExitCode::from(5)
            },
        }
    }
    if let Err(err) = nix::unistd::execvp(cmd_args[0].as_ref(), &cmd_args) {
        error!("execvp: {}", err);
        return std::process::ExitCode::from(4)
    }
    return std::process::ExitCode::SUCCESS
}
