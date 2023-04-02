use std::{vec, ffi::CString, io::Write};

use clap::Parser;
use nix::{sys::{signal::{SigAction, SigHandler, SaFlags, sigaction, self}, signalfd::SigSet}, libc::{STDIN_FILENO, pause}, unistd::{fork, ForkResult}};

#[macro_use]
extern crate log;

/// Handler for Child Process
extern fn signal_handler(_sig: nix::libc::c_int) {
    let pid = nix::unistd::Pid::from_raw(-1);
    let mut waitpid_options = nix::sys::wait::WaitPidFlag::empty();
    waitpid_options.insert(nix::sys::wait::WaitPidFlag::WNOHANG);
    waitpid_options.insert(nix::sys::wait::WaitPidFlag::WUNTRACED);
    waitpid_options.insert(nix::sys::wait::WaitPidFlag::WCONTINUED);
    loop {
        let wait_status = nix::sys::wait::waitpid(pid, Some(waitpid_options));
        match wait_status {
            Err(err) => {
                if err.eq(&nix::errno::Errno::ECHILD) {
                    break;
                } else {
                    error!("got error while wait child: {}", err.to_string());
                    std::process::exit(1)
                }
            },
            Ok(status) => {
                let status_pid = status.pid().unwrap();
                debug!("[init]: SIGCHLD handler: PID {} terminated", status_pid)
            },
        }
    }
}

/// Expand Command Input to CString Vector
fn expand_words(_input: &str) -> Option<Vec<CString>> {
    let mut arg_vec = vec![];
    if _input.len() == 0 {
        return None // If _input command does not provided
    }
    let split_result = _input.split_whitespace();
    for s in split_result {
        arg_vec.push(CString::new(s.to_string()).unwrap())
    }
    Some(arg_vec)
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    verbose: bool
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();
    // Before Initialize Logger, Set LogLevel to DEBUG
    // When Verbose flag is True
    if args.verbose {
        std::env::set_var("RUST_LOG", "DEBUG")
    }
    // then Initialize Logger
    env_logger::init();
    let mut sa_flags = SaFlags::empty();
    sa_flags.insert(SaFlags::SA_RESTART);
    sa_flags.insert(SaFlags::SA_NOCLDSTOP);
    let sa = SigAction::new(
        SigHandler::Handler(signal_handler),
        sa_flags,
        SigSet::empty(),
    );
    unsafe {
        if let Err(err) = sigaction(nix::sys::signal::SIGCHLD, &sa) {
            error!("sigaction: {}", err);
            return std::process::ExitCode::from(2)
        }
        if let Err(err) = signal::signal(signal::Signal::SIGTTOU, SigHandler::SigIgn) {
            error!("signal: {}", err);
            return std::process::ExitCode::from(3)
        }
    }
    if let Err(err) = nix::unistd::setpgid(
        nix::unistd::Pid::from_raw(0),
        nix::unistd::Pid::from_raw(0),
    ) {
        error!("setpgid: {}", err);
        return std::process::ExitCode::from(3)
    }
    if let Err(err) = nix::unistd::tcsetpgrp(
        STDIN_FILENO,
        nix::unistd::getpgrp(),
    ) {
        error!("tcsetpgrp-child: {}", err);
        return std::process::ExitCode::from(3)
    }
    loop {
        print!("$init: ");
        std::io::stdout().flush().unwrap();
        let mut buffer = String::new();
        let stdin_result = std::io::stdin().read_line(&mut buffer);
        match stdin_result {
            Ok(_) => {
                if buffer.trim() == "exit" {
                    return std::process::ExitCode::SUCCESS;
                }
                if buffer.trim().len() == 0 {
                    continue;
                }
            },
            Err(err) => {
                error!("got error while read stdin: {}", err);
                return std::process::ExitCode::from(4)
            },
        }
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                debug!("created child: {}", child);
                unsafe {
                    pause();
                    if let Err(err) = nix::unistd::tcsetpgrp(
                        STDIN_FILENO,
                        nix::unistd::getpgrp(),
                    ) {
                        error!("tcsetpgrp-parent: {}", err);
                        std::process::exit(6);
                    }
                }
            },
            Ok(ForkResult::Child) => {
                let arg_vec = expand_words(buffer.as_ref());
                match arg_vec {
                    Some(val) => {
                        if let Err(err) = nix::unistd::setpgid(
                            nix::unistd::Pid::from_raw(0),
                            nix::unistd::Pid::from_raw(0)
                        ) {
                            error!("setpgid: {}", err);
                            std::process::exit(6)
                        }
                        if let Err(err) = nix::unistd::tcsetpgrp(
                            STDIN_FILENO,
                            nix::unistd::getpgrp(),
                        ) {
                            error!("tcsetpgrp-child: {}", err);
                            std::process::exit(6);
                        }
                        if let Err(err) = nix::unistd::execvp(val[0].as_ref(), &val) {
                            error!("execvp: {}", err)
                        }
                    },
                    None => {
                        continue;
                    }
                }
            },
            Err(err) => {
                error!("fork failed: {}", err);
                return std::process::ExitCode::from(5)
            },
        }
    }
}
