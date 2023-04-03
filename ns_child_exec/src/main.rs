use std::{usize, ffi::CString};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use IPC Namespace
    #[arg(long, default_value_t = false)]
    ipc: bool,
    /// Use Mount Namespace
    #[arg(long, default_value_t = false)]
    mount: bool,
    /// Use Network Namespace
    #[arg(long, default_value_t = false)]
    network: bool,
    /// Use PID Namespace
    #[arg(long, default_value_t = false)]
    pid: bool,
    /// Use UTS Namespace
    #[arg(long, default_value_t = false)]
    uts: bool,
    /// Use USER Namespace
    #[arg(long, default_value_t = false)]
    user: bool,
    /// Display Verbose Messages
    #[arg(long, default_value_t = false)]
    verbose: bool,
    /// CMD
    command: String,
    #[clap(required = false)]
    args: Option<Vec<String>>
}

const STACK_SIZE: usize = 1024 * 1024;

/// Child function
fn child_func (command: &str, args: &Option<Vec<String>>) -> isize {
    let mut command_vec = vec![];
    command_vec.push(CString::new(command).unwrap());
    if let Some(arguments) = args {
        for arg in arguments {
            command_vec.push(CString::new(arg.as_str()).unwrap())
        }
    }
    if let Err(err) = nix::unistd::execvp(command_vec[0].as_ref(), &command_vec) {
        println!("execvp: {}", err);
        return 1
    }
    0 // return success
}

fn prepare_clone_flags(a: &Args) -> nix::sched::CloneFlags {
    let mut clone_flags: nix::sched::CloneFlags = nix::sched::CloneFlags::empty();
    let info = a.clone();
    if info.ipc {
        clone_flags |= nix::sched::CloneFlags::CLONE_NEWIPC
    }
    if info.mount {
        clone_flags |= nix::sched::CloneFlags::CLONE_NEWNS
    }
    if info.network {
        clone_flags |= nix::sched::CloneFlags::CLONE_NEWNET
    }
    if info.pid {
        clone_flags |= nix::sched::CloneFlags::CLONE_NEWPID
    }
    if info.user {
        clone_flags |= nix::sched::CloneFlags::CLONE_NEWUSER
    }
    clone_flags
}

fn main() -> std::process::ExitCode {
    let _args = Args::parse();
    let clone_flags = prepare_clone_flags(&_args);
    let signal = Some(nix::sys::signal::SIGCHLD as i32);
    let mut child_stack = vec![0; STACK_SIZE];
    let child_handler = Box::new(|| child_func(&_args.command, &_args.args));
    let child_pid = match nix::sched::clone(child_handler, child_stack.as_mut_slice(), clone_flags, signal) {
        Ok(pid) => {
            println!("PID = {}", pid);
            pid
        },
        Err(err) => {
            println!("got error while clone: {}", err);
            return std::process::ExitCode::from(1)
        }
    };
    if let Err(err) = nix::sys::wait::waitpid(child_pid, None) {
        println!("got error while wait PID {}: {}", child_pid, err);
        return std::process::ExitCode::from(2)
    }
    if _args.verbose {
        println!("{}: terminating", _args.command)
    }
    std::process::ExitCode::SUCCESS
}
