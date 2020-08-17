use std::os::unix::process::CommandExt as _;
use std::process::Command;

use nix::sys::ptrace;
use nix::unistd::{fork, ForkResult, Pid};

/// Finds the base address at which a PIC binary was loaded.
fn binary_base_address(child: Pid) -> u64 {
    let binary_path = std::fs::read_link(format!("/proc/{}/exe", child))
        .expect("failed to retrieve path of traced binary");
    let binary_name = binary_path
        .to_str()
        .expect("binary name is not valid UTF-8");

    let maps_path = format!("/proc/{}/maps", child);
    let mappings = std::fs::File::open(maps_path).expect("failed to open memory mappings file");

    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(mappings);
    for line in reader.lines().map(Result::unwrap) {
        if line.contains(binary_name) && line.contains("r-xp") {
            let base_address = line
                .split('-')
                .next()
                .expect("memory mapping file has invalid format");
            let base_address = u64::from_str_radix(base_address, 16)
                .expect("base address is not valid hexadecimal");
            return base_address;
        }
    }
    panic!("unable to find executable base address")
}

fn read_symbols(child: Pid) {
    let image_path = format!("/proc/{}/exe", child);
    let image = std::fs::File::open(&image_path).expect("failed to open traced binary");

    let mmap = unsafe { memmap::Mmap::map(&image).unwrap() };

    let binary = goblin::elf::Elf::parse(&mmap).expect("invalid ELF binary");

    println!("ELF version {}", binary.header.e_version);

    let is_pic = binary.header.e_type == goblin::elf::header::ET_DYN;
    println!("Position Independent Code: {}", is_pic);

    let interp_state_head = binary
        .dynsyms
        .iter()
        .find(|symbol| {
            if !symbol.is_function() {
                return false;
            }
            let name = &binary.dynstrtab[symbol.st_name];
            name == "PyInterpreterState_Head"
        })
        .expect("failed to find required Python symbol");

    let offset = if is_pic {
        binary_base_address(child)
    } else {
        0
    };
    let interp_state_head_address = interp_state_head.st_value + offset;
    println!(
        "PyInterpreterState_Head real address is {:#X}",
        interp_state_head_address
    );
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

            println!("Resuming child");

            ptrace::cont(child, None).unwrap();

            let status = waitpid(child, None).unwrap();
            println!("Child succesfully exec'ed Python binary: {:?}", status);

            read_symbols(child);

            read_stack(child);

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
