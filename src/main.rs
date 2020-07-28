use nix::sys::ptrace;
use nix::unistd::{fork, ForkResult};
use std::os::unix::process::CommandExt as _;
use std::process::Command;

fn main() {
    match fork().expect("failed to fork profiler") {
        ForkResult::Parent { child } => {
            println!("I am the parent");
            println!("Child PID = {}", child);

            use nix::sys::wait::{waitpid, WaitPidFlag};
            waitpid(child, Some(WaitPidFlag::WUNTRACED)).expect("failed to wait for child");

            println!("Successfully waited for the child");

            ptrace::seize(child, ptrace::Options::PTRACE_O_TRACEEXEC)
                .expect("failed to attach (seize) the child process");

            println!("Resuming child");

            ptrace::cont(child, None).unwrap();

            let status = waitpid(child, None).unwrap();
            println!("{:?}", status);

            ptrace::cont(child, None).unwrap();

            let status = waitpid(child, None).unwrap();
            println!("{:?}", status);

            ptrace::cont(child, None).unwrap();

            println!("Sleeping...");

            std::thread::sleep(std::time::Duration::from_secs(5));

            ptrace::interrupt(child).unwrap();
            println!("child is stopped");

            std::thread::sleep(std::time::Duration::from_secs(5));

            ptrace::cont(child, None).unwrap();

            println!("child is restarted");

            waitpid(child, None).unwrap();
            println!("Child exited");
        }
        ForkResult::Child => {
            println!("I am the child");

            use nix::sys::signal::{raise, Signal};
            raise(Signal::SIGSTOP).expect("failed to stop child process");

            println!("Executing profiled process");

            let error = Command::new("python3").arg("mandelbrot.py").exec();

            panic!("failed to execute tracee: {}", error);
        }
    }
}
