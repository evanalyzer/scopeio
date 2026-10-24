//! # measure
//!
//! Runs a command and reports its CPU time and peak resident memory.
//!
//! Linux keeps the peak RSS of a process across `fork` and `exec`, so a
//! child of a large parent (e.g. the Python test scripts) can never report
//! less than the parent. This helper is small, so measurements start from
//! about its own footprint. Prints one line to stderr after the command
//! exits: `@measure cpu_s=<s> maxrss_kib=<KiB>`, and exits with the
//! command's exit code. Built by `tests/scripts/oracle.py` with `rustc`.
//!
//! Usage: `measure <command> [args...]`
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::process::Command;

#[repr(C)]
struct Timeval {
    sec: i64,
    usec: i64,
}

/// `struct rusage` on 64-bit Linux.
#[repr(C)]
struct Rusage {
    utime: Timeval,
    stime: Timeval,
    maxrss: i64,
    other: [i64; 13],
}

unsafe extern "C" {
    fn wait4(pid: i32, status: *mut i32, options: i32, usage: *mut Rusage) -> i32;
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("usage: measure <command> [args...]");
        std::process::exit(2);
    };
    let child = Command::new(cmd).args(rest).spawn().unwrap_or_else(|e| {
        eprintln!("measure: cannot run {cmd}: {e}");
        std::process::exit(127);
    });
    let mut status = 0;
    let mut usage = Rusage { utime: Timeval { sec: 0, usec: 0 }, stime: Timeval { sec: 0, usec: 0 }, maxrss: 0, other: [0; 13] };
    // SAFETY: `status` and `usage` are valid for writes; the child has not
    // been waited for yet, so its pid is still valid.
    let r = unsafe { wait4(child.id() as i32, &mut status, 0, &mut usage) };
    if r < 0 {
        eprintln!("measure: wait4 failed: {}", std::io::Error::last_os_error());
        std::process::exit(1);
    }
    let secs = |t: &Timeval| t.sec as f64 + t.usec as f64 / 1e6;
    eprintln!("@measure cpu_s={:.6} maxrss_kib={}", secs(&usage.utime) + secs(&usage.stime), usage.maxrss);
    let code = if status & 0x7F == 0 { (status >> 8) & 0xFF } else { 128 + (status & 0x7F) };
    std::process::exit(code);
}
