use std::os::unix::process::CommandExt as _;
use std::process::Command;

use nix::sys::ptrace;
use nix::unistd::{fork, ForkResult, Pid};

fn read_symbols(child: Pid) {
    let binary_path = format!("/proc/{}/exe", child);
    let binary = std::fs::File::open(binary_path).expect("failed to open traced binary");

    let mmap = unsafe { memmap::Mmap::map(&binary).unwrap() };

    let binary = goblin::elf::Elf::parse(&mmap).expect("invalid ELF binary");

    println!("ELF version {}", binary.header.e_version);
}

fn read_stack(child: Pid) {
    let registers = ptrace::getregs(child).expect("failed to read child registers");
    let stack_pointer = registers.rsp;
    println!("Child stack pointer is at: {:#X}", stack_pointer);

    use nix::sys::uio::{process_vm_readv, IoVec, RemoteIoVec};
    let mut buffer = [0u8; 4096];
    process_vm_readv(
        child,
        &[IoVec::from_mut_slice(&mut buffer[..])],
        &[RemoteIoVec {
            base: stack_pointer as usize,
            len: 4096,
        }],
    )
    .expect("failed to read child stack");

    println!("Top of child stack: {}", buffer[0]);
}

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

            read_symbols(child);

            read_stack(child);

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
