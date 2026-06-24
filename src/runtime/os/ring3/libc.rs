//! Libc — standard library syscall wrappers (printf, malloc, open).

use std::collections::HashMap;

pub struct Libc {
    fds: HashMap<i32, String>,
    next_fd: i32,
    mallocs: u64,
}

impl Default for Libc {
    fn default() -> Self {
        Self {
            fds: HashMap::new(),
            next_fd: 3,
            mallocs: 0,
        }
    }
}

impl Libc {
    pub fn open(&mut self, path: &str) -> i32 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.fds.insert(fd, path.to_string());
        fd
    }

    pub fn malloc(&mut self, size: usize) -> u64 {
        self.mallocs += 1;
        0x4000_0000 + self.mallocs * 64 + size as u64
    }

    pub fn printf_fmt(&self, fmt: &str, args: &[String]) -> String {
        if args.is_empty() {
            return fmt.to_string();
        }
        let mut out = fmt.to_string();
        for (i, a) in args.iter().enumerate() {
            out = out.replace(&format!("{{{i}}}"), a);
        }
        out
    }
}
