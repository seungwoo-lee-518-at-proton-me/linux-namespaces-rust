mod grpc_handler;
mod grpc_client;
use std::ffi::CString;

use clap::{Parser, Subcommand, Args};
use tokio::{process::Command, net::UnixListener};
use tokio_stream::wrappers::UnixListenerStream;

use crate::grpc_handler::{UsernsMapperImpl, userns::userns_mapper_server::UsernsMapperServer};

#[macro_use]
extern crate log;

#[macro_use]
extern crate scopeguard;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = false)]
struct CLI {
    #[command(subcommand)]
    command: Option<Commands>,
    /// New IPC Namespace
    #[arg(short, long, default_value_t = false)]
    ipc: bool,
    /// New Mount Namespace
    #[arg(short, long, default_value_t = false)]
    mount: bool,
    /// New Network Namespace
    #[arg(short, long, default_value_t = false)]
    network: bool,
    /// New PID Namespace
    #[arg(short, long, default_value_t = false)]
    pid: bool,
    /// New UTS Namespace
    #[arg(short, long, default_value_t = false)]
    uts: bool,
    /// New USER Namespace
    #[arg(short = 'U', long, default_value_t = false)]
    user: bool,
    /// Display Verbose Messages
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Debug)]
struct ChildArgs {
    /// New IPC Namespace
    #[arg(short, long, default_value_t = false)]
    ipc: bool,
    /// New Mount Namespace
    #[arg(short, long, default_value_t = false)]
    mount: bool,
    /// New Network Namespace
    #[arg(short, long, default_value_t = false)]
    network: bool,
    /// New PID Namespace
    #[arg(short, long, default_value_t = false)]
    pid: bool,
    /// New UTS Namespace
    #[arg(short, long, default_value_t = false)]
    uts: bool,
    /// New USER Namespace
    #[arg(short = 'U', long, default_value_t = false)]
    user: bool,
    /// Command
    cmd: Option<String>,
    /// Arguments
    args: Option<Vec<String>>
}

#[derive(Subcommand)]
enum Commands {
    /// Executes Child function
    /// 
    /// It shouldn't executed without server
    Child (ChildArgs)
}

const STACK_SIZE: usize = 1024 * 1024;

fn child_func (command: Option<String>, args: Option<Vec<String>>, id: nix::unistd::Uid) -> isize {
    let mut exec_args = vec![];
    let mut client = match grpc_client::UsernsClient::connect() {
        Ok(client) => client,
        Err(err) => {
            println!("got error while connect: {}", err);
            return 1
        }
    };
    if let Err(err) = client.ping() {
        println!("got error while ping: {}", err);
        return 2
    }
    if let Err(err) = client.map_gid(nix::unistd::getpid(), id) {
        println!("got error while map gid: {}", err);
        return 3
    }
    if let Err(err) = client.map_uid(nix::unistd::getpid(), id) {
        println!("got error while map uid: {}", err);
        return 3
    }
    if let Some(cmd) = command {
        exec_args.push(CString::new(cmd).unwrap())
    }
    if let Some(args_arr) = args {
        for a in args_arr {
            exec_args.push(CString::new(a).unwrap())
        }
    }
    if let Err(err) = nix::unistd::execvp(&exec_args[0], &exec_args) {
        println!("execvp: {}", err);
        return 4;
    }
    0
}

fn main() -> std::process::ExitCode {
    let _cli = CLI::parse();
    match &_cli.command {
        Some(Commands::Child (ChildArgs { ipc, mount, network, pid, uts, user, cmd, args })) => {
            // Set Verbose Mode
            std::env::set_var("RUST_LOG", "DEBUG");
            env_logger::init();
            let mut clone_flags = nix::sched::CloneFlags::empty();
            if *ipc {
                clone_flags |= nix::sched::CloneFlags::CLONE_NEWIPC;
            }
            if *mount {
                clone_flags |= nix::sched::CloneFlags::CLONE_NEWNS;
            }
            if *network {
                clone_flags |= nix::sched::CloneFlags::CLONE_NEWNET;
            }
            if *pid {
                clone_flags |= nix::sched::CloneFlags::CLONE_NEWPID;
            }
            if *uts {
                clone_flags |= nix::sched::CloneFlags::CLONE_NEWUTS;
            }
            if *user {
                clone_flags |= nix::sched::CloneFlags::CLONE_NEWUSER;
            }
            let id = nix::unistd::geteuid();
            let mut child_stack = vec![0; STACK_SIZE];
            let cb_func = Box::new(|| {
                child_func(cmd.clone(), args.clone(), id.clone())
            });
            let pid = match nix::sched::clone(cb_func, &mut child_stack, clone_flags, Some(nix::sys::signal::SIGCHLD as i32)) {
                Ok(pid) => pid,
                Err(err) => {
                    error!("got error while clone: {}", err);
                    return std::process::ExitCode::from(1)
                }
            };
            if let Err(err) = nix::sys::wait::waitpid(pid, None) {
                error!("waitpid: {}", err);
                return std::process::ExitCode::from(2)
            }
            return std::process::ExitCode::SUCCESS;
        },
        None => {
            return tokio::runtime::Builder::new_multi_thread().
                enable_all().
                build().
                unwrap().
                block_on(async {
                    if _cli.verbose {
                        std::env::set_var("RUST_LOG", "DEBUG");
                    }
                    defer! {
                        info!("clean up /tmp/userns.sock");
                        if let Err(err) = std::fs::remove_file("/tmp/userns.sock") {
                            error!("got error while delete /tmp/userns.sock: {}", err)
                        }
                    }
                    env_logger::init();
                    debug!("spawn grpc server");
                    let grpc_server = tonic::transport::Server::builder().add_service(UsernsMapperServer::new(UsernsMapperImpl::default()));
                    let grpc_socket = match UnixListener::bind("/tmp/userns.sock") {
                        Ok(sock) => sock,
                        Err(err) => {
                            error!("got error while bind /tmp/userns.sock: {}", err);
                            return std::process::ExitCode::from(2);
                        },
                    };
                    let _grpc_task = tokio::task::spawn(async move {
                        if let Err(err) = grpc_server.serve_with_incoming(UnixListenerStream::new(grpc_socket)).await {
                            panic!("got error while serve: {}", err)
                        }
                    });
                    defer! {
                        _grpc_task.abort()
                    }
                    let mut command = Command::new("/proc/self/exe");
                    command.arg("child");
                    if _cli.ipc {
                        command.arg("--ipc");
                    };
                    if _cli.mount {
                        command.arg("--mount");
                    };
                    if _cli.network {
                        command.arg("--network");
                    };
                    if _cli.pid {
                        command.arg("--pid");
                    };
                    if _cli.uts {
                        command.arg("--uts");
                    };
                    if _cli.user {
                        command.arg("--user");
                    };
                    command.arg("zsh");
                    let mut child = match command.spawn() {
                        Ok(child) => child,
                        Err(err) => {
                            error!("err: {}", err);
                            return std::process::ExitCode::from(1)
                        }
                    };
                    match child.wait().await {
                        Ok(exitcode) => {
                            if exitcode.success() {
                                return std::process::ExitCode::SUCCESS
                            }
                            return std::process::ExitCode::from(1)
                        },
                        Err(err) => {
                            error!("wait(): {}", err);
                            return std::process::ExitCode::from(1)
                        }
                    }
                })
        }
    }
}
