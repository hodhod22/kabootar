//! IRQ handler — hardware interrupt prioritization.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct IrqLine {
    pub irq: u8,
    pub device: String,
    pub priority: u8,
}

pub struct IrqHandler {
    queue: VecDeque<IrqLine>,
    handled: u64,
}

impl Default for IrqHandler {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            handled: 0,
        }
    }
}

impl IrqHandler {
    pub fn dispatch(&mut self, irq: u8, device: &str) {
        let priority = match irq {
            0..=15 => 3,
            16..=31 => 2,
            _ => 1,
        };
        self.queue.push_back(IrqLine {
            irq,
            device: device.to_string(),
            priority,
        });
        self.handled += 1;
    }

    pub fn poll(&mut self) -> Option<IrqLine> {
        self.queue.pop_front()
    }

    pub fn handled_count(&self) -> u64 {
        self.handled
    }
}
