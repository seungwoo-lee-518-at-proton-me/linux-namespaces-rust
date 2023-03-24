use core::time;

use nix::sched::clone;
use nix::sys::wait::waitpid;

fn child_func() -> isize {
    const HOSTNAME_VALUE: &str = "hellowed";
    println!("set hostname as hellowed");
    if let Err(result) = nix::unistd::sethostname(HOSTNAME_VALUE) {
        println!("got error while set hostname: {}", result);
        return 1
    }
    println!("get hostname");
    let hostname_result = nix::unistd::gethostname();
    match hostname_result {
        Ok(hostname) => {
            println!("hostname is: {}", hostname.to_str().unwrap());
            std::thread::sleep(time::Duration::from_secs(100));
            return 0
        },
        Err(_) => {
            println!("got error");
            return 1
        },
    };
}

fn main() -> std::process::ExitCode {
    const STACK_SIZE: usize = 1024 * 1024;
    let mut stack = vec![0; STACK_SIZE];
    let flags = nix::sched::CloneFlags::CLONE_NEWUTS;
    let child_pid_result = clone(Box::new(child_func), stack.as_mut_slice(), flags, Some(nix::sys::signal::SIGCHLD as i32));
    match child_pid_result {
        Ok(pid_info) => {
            println!("process has been created as pid {}", pid_info.to_string());
            // sleep for 1 seconds
            std::thread::sleep(time::Duration::from_secs(1));
            match nix::unistd::gethostname() {
                Ok(hostname) => {
                    println!("parent hostname is: {}", hostname.to_str().unwrap())
                },
                Err(err) => {
                    println!("got error while get parent hostname: {}", err)
                }
            }
            if let Err(waitpid_err) = waitpid(pid_info, None) {
                println!("got error while wait: {}", waitpid_err);
                return std::process::ExitCode::from(3)
            }
            println!("child has been terminated");
        },
        Err(err) => {
            println!("got error while clone: {}", err);
            return std::process::ExitCode::from(2)
        }
    }
    std::process::ExitCode::SUCCESS
}
