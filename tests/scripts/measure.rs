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
//! The command runs in its own process group. A watchdog sums the resident
//! memory of all processes in the group (launcher scripts start the real
//! reader as a grandchild) and kills the group once it exceeds
//! `MEASURE_MAX_RSS_MIB` (default 8192), so a reference reader that
//! misjudges a file cannot exhaust the machine. The report line then ends
//! with ` killed=rss`.
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

use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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
    fn kill(pid: i32, sig: i32) -> i32;
    fn sysconf(name: i32) -> i64;
}

const SIGKILL: i32 = 9;
const SC_PAGESIZE: i32 = 30;

/// Resident bytes of all processes in process group `pgid`.
fn group_rss(pgid: i32, page: u64) -> u64 {
    let Ok(dir) = std::fs::read_dir("/proc") else { return 0 };
    let mut total = 0;
    for entry in dir.flatten() {
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else { continue };
        // Fields after the parenthesised command name: state, ppid, pgrp, ..., rss (24th overall).
        let Some(rest) = stat.rsplit_once(')').map(|(_, r)| r) else { continue };
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.get(2).and_then(|g| g.parse::<i32>().ok()) == Some(pgid) {
            total += fields.get(21).and_then(|r| r.parse::<u64>().ok()).unwrap_or(0) * page;
        }
    }
    total
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("usage: measure <command> [args...]");
        std::process::exit(2);
    };
    let limit_mib: u64 = std::env::var("MEASURE_MAX_RSS_MIB").ok().and_then(|v| v.parse().ok()).unwrap_or(8192);
    let child = Command::new(cmd).args(rest).process_group(0).spawn().unwrap_or_else(|e| {
        eprintln!("measure: cannot run {cmd}: {e}");
        std::process::exit(127);
    });
    let pid = child.id() as i32;
    let killed = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    {
        let (killed, done) = (killed.clone(), done.clone());
        // SAFETY: sysconf has no preconditions.
        let page = unsafe { sysconf(SC_PAGESIZE) }.max(4096) as u64;
        std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                if group_rss(pid, page) > limit_mib << 20 {
                    killed.store(true, Ordering::Relaxed);
                    // SAFETY: plain syscall; a negative pid addresses the child's group.
                    unsafe { kill(-pid, SIGKILL) };
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
    }
    let mut status = 0;
    let mut usage = Rusage { utime: Timeval { sec: 0, usec: 0 }, stime: Timeval { sec: 0, usec: 0 }, maxrss: 0, other: [0; 13] };
    // SAFETY: `status` and `usage` are valid for writes; the child has not
    // been waited for yet, so its pid is still valid.
    let r = unsafe { wait4(pid, &mut status, 0, &mut usage) };
    done.store(true, Ordering::Relaxed);
    if r < 0 {
        eprintln!("measure: wait4 failed: {}", std::io::Error::last_os_error());
        std::process::exit(1);
    }
    let secs = |t: &Timeval| t.sec as f64 + t.usec as f64 / 1e6;
    let note = if killed.load(Ordering::Relaxed) { " killed=rss" } else { "" };
    eprintln!("@measure cpu_s={:.6} maxrss_kib={}{note}", secs(&usage.utime) + secs(&usage.stime), usage.maxrss);
    let code = if status & 0x7F == 0 { (status >> 8) & 0xFF } else { 128 + (status & 0x7F) };
    std::process::exit(code);
}
