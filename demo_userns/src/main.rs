use clap::Parser;
use nix::sched::{clone, CloneFlags};
use nix::unistd::{geteuid, getegid};
use nix::sys::wait::waitpid;
use caps::CapSet;

#[macro_use]
extern crate log;

const STACK_SIZE: usize = 1024 * 1024;

pub fn child_func(args: Option<Vec<String>>) -> isize {
    loop {
        info!("eUID = {}; eGID = {}", geteuid(), getegid());
        let cur_caps = caps::read(None, CapSet::Permitted);
        info!("capabilities: {:?}", cur_caps);
        if args.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    return 0
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    args: Option<Vec<String>>
}

fn main() -> std::process::ExitCode {
    env_logger::init();
    let args = Args::parse();
    let cb = Box::new(|| {
        child_func(args.args.clone())
    });
    let mut child_stack = vec![0; STACK_SIZE];
    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWUSER);
    let pid = match clone(cb, &mut child_stack, flags, Some(nix::sys::signal::SIGCHLD as i32)) {
        Ok(pid) => pid,
        Err(err) => {
            error!("clone: {}", err);
            return std::process::ExitCode::from(1)
        },
    };
    if let Err(err) = waitpid(pid, None) {
        error!("waitpid ({}): {}", pid, err);
        return std::process::ExitCode::from(2)
    }
    return std::process::ExitCode::SUCCESS;
}
