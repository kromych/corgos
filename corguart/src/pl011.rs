//! UART PL011 driver implementation.
//!
//! Can run in an interrupt-free single thread environment only.

//! PL011 Registers:
//!
//! Offset  Name              Type Reset        Bits    Description
//! ----------------------------------------------------------------------
//! 0x000   UARTDR            RW   0x---        12/8    Data Register
//! 0x004   UARTRSR/UARTECR   RW   0x0          4/0     Receive Status Register/Error Clear Register
//! 0x018   UARTFR            RO   0b-10010---  9       Flag Register
//! 0x020   UARTILPR          RW   0x00         8       IrDA Low-Power Counter Register
//! 0x024   UARTIBRD          RW   0x0000       16      Integer Baud Rate Register
//! 0x028   UARTFBRD          RW   0x00         6       Fractional Baud Rate Register
//! 0x02C   UARTLCR_H         RW   0x00         8       Line Control Register
//! 0x030   UARTCR            RW   0x0300       16      Control Register
//! 0x034   UARTIFLS          RW   0x12         6       Interrupt FIFO Level Select Register
//! 0x038   UARTIMSC          RW   0x000        11      Interrupt Mask Set/Clear Register
//! 0x03C   UARTRIS           RO   0x00-        11      Raw Interrupt Status Register
//! 0x040   UARTMIS           RO   0x00-        11      Masked Interrupt Status Register
//! 0x044   UARTICR           WO   -            11      Interrupt Clear Register
//! 0x048   UARTDMACR         RW   0x00         3       DMA Control Register
//! 0xFE0   UARTPeriphID0     RO   0x11         8       UARTPeriphID0 Register
//! 0xFE4   UARTPeriphID1     RO   0x10         8       UARTPeriphID1 Register
//! 0xFE8   UARTPeriphID2     RO   0x_4a        8       UARTPeriphID2 Register
//! 0xFEC   UARTPeriphID3     RO   0x00         8       UARTPeriphID3 Register
//! 0xFF0   UARTPCellID0      RO   0x0D         8       UARTPCellID0 Register
//! 0xFF4   UARTPCellID1      RO   0xF0         8       UARTPCellID1 Register
//! 0xFF8   UARTPCellID2      RO   0x05         8       UARTPCellID2 Register
//! 0xFFC   UARTPCellID3      RO   0xB1         8       UARTPCellID3 Register

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
enum Pl011Register {
    /// Data Register
    Dr = 0x000,
    /// Receive Status Register/Error Clear Register
    RsrOrEcr = 0x004,
    /// Flag register
    Fr = 0x018,
    /// Integer Baud Rate Register
    Ibrd = 0x024,
    /// Fractional Baud Rate Register
    Fbrd = 0x028,
    /// Line Control Register
    LcrHigh = 0x02c,
    /// Control Register
    Cr = 0x030,
    /// Masked Interrupt Status Register
    Imsc = 0x038,
    /// Interrupt Clear Register
    Icr = 0x044,
    /// DMA Control Register
    DmaCr = 0x048,
    /// UARTPeriphID0 Register
    PeriphID0 = 0xFE0,
    /// UARTPeriphID1 Register
    PeriphID1 = 0xFE4,
    /// UARTPeriphID2 Register
    PeriphID2 = 0xFE8,
    /// UARTPeriphID3 Register
    PeriphID3 = 0xFEC,
    /// UARTPCellID0 Register
    PCellID0 = 0xFF0,
    /// UARTPCellID1 Register
    PCellID1 = 0xFF4,
    /// UARTPCellID2 Register
    PCellID2 = 0xFF8,
    /// UARTPCellID3 Register
    PCellID3 = 0xFFC,
}

const CR_RX_ENABLE: u32 = 1 << 9;
const CR_TX_ENABLE: u32 = 1 << 8;
const CR_UART_ENABLE: u32 = 1;
const LCR_H_FIFO_EN: u32 = 1 << 4;
const LCR_H_8BITS: u32 = 3 << 5;
const FR_TX_EMPTY: u32 = 1 << 7;
const FR_RX_EMPTY: u32 = 1 << 4;
const FR_UART_BUSY: u32 = 1 << 3;

/// PL011 UART.
#[derive(Debug, Clone, Copy)]
pub struct Pl011 {
    base_addr: u64,
    id: u64,
}

fn pl011_id(pl011: &Pl011) -> u64 {
    // This can easily be rewritten employing
    // bare ariphmetic yet the compiler does a very good job
    // so using the domain abstractions.
    [
        Pl011Register::PeriphID0,
        Pl011Register::PeriphID1,
        Pl011Register::PeriphID2,
        Pl011Register::PeriphID3,
        Pl011Register::PCellID0,
        Pl011Register::PCellID1,
        Pl011Register::PCellID2,
        Pl011Register::PCellID3,
    ]
    .iter()
    .fold(0, |id_running, &r| {
        id_running.wrapping_shl(8) | (pl011_read(pl011, r) as u8 as u64)
    })
}

/// Disables the functional parts of the UART, drains FIFOs,
/// sets baud rate and enables the UART in the polling mode.
fn pl011_init(pl011: &mut Pl011) {
    pl011.id = pl011_id(pl011);

    // Mask interrupts
    pl011_write(pl011, Pl011Register::Imsc, 0x000);
    // Disable interrupts (lower 11 bits)
    pl011_write(pl011, Pl011Register::Icr, 0x7ff);
    // Disable DMA on Rx and Tx
    pl011_write(pl011, Pl011Register::DmaCr, 0x0);

    // Leave Rx and Tx enabled to drain FIFOs.
    pl011_write(pl011, Pl011Register::Cr, CR_RX_ENABLE | CR_TX_ENABLE);
    pl011_read(pl011, Pl011Register::Cr); // wait
    pl011_read(pl011, Pl011Register::Cr); // wait
    pl011_poll_busy(pl011);

    // Disable Rx, Tx, and UART.
    pl011_write(pl011, Pl011Register::Cr, 0x00000000);

    // Set integer and fractional parts of the baud rate,
    // harcoded for now
    pl011_write(pl011, Pl011Register::Fbrd, 0x00000004);
    pl011_write(pl011, Pl011Register::Ibrd, 0x00000027);
    // The UARTLCR_H, UARTIBRD, and UARTFBRD registers form the single 30-bit
    // wide UARTLCR Register that is updated on a single write strobe generated by a
    // UARTLCR_H write
    pl011_write(pl011, Pl011Register::LcrHigh, LCR_H_FIFO_EN | LCR_H_8BITS);

    // Clear the errors
    pl011_write(pl011, Pl011Register::RsrOrEcr, 0);

    // Enable Tx and Rx
    pl011_write(pl011, Pl011Register::Cr, CR_RX_ENABLE | CR_TX_ENABLE);
    pl011_read(pl011, Pl011Register::Cr); // wait
    pl011_read(pl011, Pl011Register::Cr); // wait
    pl011_poll_busy(pl011);

    // Enable UART
    pl011_write(
        pl011,
        Pl011Register::Cr,
        CR_RX_ENABLE | CR_TX_ENABLE | CR_UART_ENABLE,
    );
    pl011_poll_busy(pl011);
}

fn pl011_read(pl011: &Pl011, reg: Pl011Register) -> u32 {
    unsafe { core::ptr::read_volatile((pl011.base_addr + reg as u64) as *const u32) }
}

fn pl011_write(pl011: &mut Pl011, reg: Pl011Register, val: u32) {
    unsafe {
        core::ptr::write_volatile((pl011.base_addr + reg as u64) as *mut u32, val);
    }
}

fn pl011_poll_tx_rx_empty(pl011: &Pl011) {
    while pl011_read(pl011, Pl011Register::Fr) != (FR_TX_EMPTY | FR_RX_EMPTY) {}
}

fn pl011_poll_busy(pl011: &Pl011) {
    while pl011_read(pl011, Pl011Register::Fr) & FR_UART_BUSY != 0 {}
}

impl Pl011 {
    pub fn new(base_addr: u64) -> Pl011 {
        let mut pl011 = Self { base_addr, id: !0 };
        pl011_init(&mut pl011);
        pl011
    }

    pub fn send_byte(&mut self, byte: u8) {
        pl011_poll_tx_rx_empty(self);
        pl011_write(self, Pl011Register::Dr, byte.into());
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl core::fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.send_byte(byte);
        }
        Ok(())
    }
}
