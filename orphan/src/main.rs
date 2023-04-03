use core::time;

#[macro_use]
extern crate log;

fn main() -> std::process::ExitCode {
    std::env::set_var("RUST_LOG", "DEBUG");
    env_logger::init();
    let ppid_orig = nix::unistd::getpid();
    match unsafe { nix::unistd::fork() } {
        Ok(nix::unistd::ForkResult::Parent { child }) => {
            let current_pid = nix::unistd::getpid();
            let p_parent_pid = nix::unistd::getppid();
            info!("Parent (PID={}) created child with PID {}", current_pid, child);
            info!("Parent (PID={}; PPID={}) terminating", current_pid, p_parent_pid);
            return std::process::ExitCode::SUCCESS;
        },
        Ok(nix::unistd::ForkResult::Child) => {
            loop {
                let ppid = nix::unistd::getppid();
                if ppid.eq(&ppid_orig) {
                    std::thread::sleep(time::Duration::from_micros(100000));
                    continue;
                } else {
                    break;
                }
            }
            let pid = nix::unistd::getpid();
            let ppid = nix::unistd::getppid();
            info!("Child (PID={}) now an orphan (parent PID={})", pid, ppid);
            std::thread::sleep(time::Duration::from_secs(1));
            info!("Child (PID={}) terminating", pid);
            return std::process::ExitCode::SUCCESS
        },
        Err(err) => {
            error!("got error while fork: {}", err);
            return std::process::ExitCode::from(1)
        },
    }
}
