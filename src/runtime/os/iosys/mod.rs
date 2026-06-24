//! I/O subsystem — driver framework, PnP, IRQ, DMA.

mod dma;
mod framework;
mod irq;
mod pnp;

pub use dma::DmaChannel;
pub use framework::{DriverFramework, DriverRegistration};
pub use irq::{IrqHandler, IrqLine};
pub use pnp::PnpManager;

pub struct IoSubsystem {
    pub framework: DriverFramework,
    pub pnp: PnpManager,
    pub irq: IrqHandler,
    pub dma: Vec<DmaChannel>,
}

impl Default for IoSubsystem {
    fn default() -> Self {
        Self {
            framework: DriverFramework::default(),
            pnp: PnpManager::default(),
            irq: IrqHandler::default(),
            dma: Vec::new(),
        }
    }
}

impl IoSubsystem {
    pub fn register_driver(&mut self, name: &str, version: &str) -> u64 {
        self.framework.register(name, version)
    }

    pub fn unregister_driver(&mut self, id: u64) -> bool {
        self.framework.unregister(id)
    }

    pub fn discover_device(&mut self, bus: &str, vid: &str, pid: &str) -> Option<String> {
        self.pnp.discover(bus, vid, pid)
    }

    pub fn raise_irq(&mut self, irq: u8, device: &str) {
        self.irq.dispatch(irq, device);
    }

    pub fn alloc_dma(&mut self, device: &str, size: usize) -> u64 {
        let ch = DmaChannel::new(self.dma.len() as u64, device, size);
        let id = ch.id;
        self.dma.push(ch);
        id
    }
}
