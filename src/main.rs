// TODO: add support for more `ptrace` functions to `nix`
#![allow(deprecated)]

use nix::sys::ptrace;
use nix::unistd::{fork, ForkResult};
use std::os::unix::process::CommandExt as _;
use std::process::Command;
use std::ptr;

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

            unsafe {
                ptrace::ptrace(
                    ptrace::Request::PTRACE_CONT,
                    child,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
                .expect("failed to resume child process");
            }

            let status = waitpid(child, None).expect("failed to wait for child");
            println!("{:?}", status);

            unsafe {
                ptrace::ptrace(
                    ptrace::Request::PTRACE_CONT,
                    child,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
                .expect("failed to resume child process");
            }

            let status = waitpid(child, None).expect("failed to wait for child");
            println!("{:?}", status);

            unsafe {
                ptrace::ptrace(
                    ptrace::Request::PTRACE_CONT,
                    child,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
                .expect("failed to resume child process");
            }

            println!("Sleeping...");

            std::thread::sleep(std::time::Duration::from_secs(5));

            unsafe {
                ptrace::ptrace(
                    ptrace::Request::PTRACE_INTERRUPT,
                    child,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
                .expect("failed to interrupt running child process");
            }

            println!("child is stopped");

            std::thread::sleep(std::time::Duration::from_secs(5));

            unsafe {
                ptrace::ptrace(
                    ptrace::Request::PTRACE_CONT,
                    child,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
                .expect("failed to resume child process");
            }

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
