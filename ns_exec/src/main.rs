use std::env;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use nix::sched::setns;
use nix::unistd::execvp;
use std::ffi::CString;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("{} /proc/pid/ns/FILE cmd arg[...]", args[0]);
        return std::process::ExitCode::from(1);
    }
    let executable_path = CString::new(args[2].as_str()).unwrap();
    let executable_path_args = [CString::new(args[2].as_str()).unwrap()];
    let file = match File::open(Path::new(args[1].as_str()).as_os_str()) {
        Err(err) => {
            println!("got error while open {}: {}", args[1].as_str(), err);
            return std::process::ExitCode::from(1);
        },
        Ok(file) => file
    };
    // println!("fd: {}", file.as_raw_fd());
    // Execute SetNS for Given Directory
    if let Err(err) = setns(file.as_raw_fd(), nix::sched::CloneFlags::CLONE_NEWUTS) {
        println!("got error while execute setns for {}: {}", args[1].to_string(), err);
        return std::process::ExitCode::from(1)
    }
    if let Err(err) = execvp(&executable_path, &executable_path_args) {
        println!("got error while execvp of {}: {}", args[2].to_string(), err);
        return std::process::ExitCode::from(2)
    }
    std::process::ExitCode::SUCCESS
}
