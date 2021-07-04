# Rust `ptrace` example

This repo contains an example of using [the `ptrace` system call](https://en.wikipedia.org/wiki/Ptrace) from Rust to start and control the execution of a child process.

## Profiling flow

The example was initially intended to become a statistic profiler. The current steps are:

- The profiler main process starts.
- The profiler forks itself.
  - The child immediately raises a `SIGSTOP`.
  - The parent waits for its child to stop using `waitpid(UNTRACED)`.
- At this point, the profiler attaches itself to the child using `ptrace`.
- Also using `ptrace`, the child process is allowed to continue execution.
- The child performs an `exec` call to start the `python3` interpreter (at this point, the child stops and the profiler is notified of this).
- Python then executes _another_ `exec` call (again the profiler is notified).
- The profiler allows the child to execute and waits for a given interval.
- After a given interval, it stops the child using `ptrace`.
