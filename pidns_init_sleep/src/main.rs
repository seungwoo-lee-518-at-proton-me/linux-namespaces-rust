use std::ffi::CString;

const STACK_SIZE: usize = 1024 * 1024;
const NONE: Option<&'static [u8]> = None;

fn child_func() -> isize {
    let pid = nix::unistd::getpid();
    println!("child_func(): PID = {}", pid.to_string());
    let parent_pid = nix::unistd::getppid();
    println!("child_func(): PPID = {}", parent_pid.to_string());
    let temp_dir = match tempfile::tempdir() {
        Ok(td) => td,
        Err(err) => {
            println!("got error while get tempdir: {}", err);
            return 1
        }
    };
    println!("mount proc to {}", temp_dir.path().to_str().unwrap());
    if let Err(err) = nix::mount::mount(Some("proc"), temp_dir.path(), Some("proc"), nix::mount::MsFlags::empty(), NONE) {
        println!("got error while mount: {}", err);
        return 1
    }
    let sleep_filename = CString::new("sleep").unwrap();
    let sleep_args = vec![CString::new("sleep").unwrap(), CString::new("600").unwrap()];
    if let Err(err) = nix::unistd::execvp(sleep_filename.as_c_str(), sleep_args.as_slice()) {
        println!("got error while execute sleep: {}", err);
        return 1
    }
    0
}

fn main() -> std::process::ExitCode {
    let mut stack = vec![0; STACK_SIZE];
    let flags = nix::sched::CloneFlags::CLONE_NEWPID;
    // Clone
    let child_pid = match nix::sched::clone(Box::new(child_func), stack.as_mut_slice(), flags, Some(nix::sys::signal::SIGCHLD as i32)) {
        Ok(pid) => pid,
        Err(err) => {
            println!("got error while clone: {}", err);
            return std::process::ExitCode::from(1);
        },
    };
    println!("PID returned by Clone(): {}", child_pid.to_string());
    // Let it wait...
    if let Err(err) = nix::sys::wait::waitpid(child_pid, None) {
        println!("got error while wait PID {}: {}", child_pid.to_string(), err);
        return std::process::ExitCode::from(2)
    }
    std::process::ExitCode::SUCCESS
}
