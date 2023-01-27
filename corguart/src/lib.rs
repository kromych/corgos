//! UART implementation

//!
//!  COM1 COM2 COM3 COM4 Offs. DLAB  Register
//!  ------------------------------------------------------------------------------
//!  3F8h 2F8h 3E8h 2E8h  +0     0   RBR  Receive Buffer Register (read only) or
//!                                  THR  Transmitter Holding Register (write only)
//!  3F9h 2F9h 3E9h 2E9h  +1     0   IER  Interrupt Enable Register
//!  3F8h 2F8h 3E8h 2E8h  +0     1   DL   Divisor Latch (LSB)  These registers can
//!  3F9h 2F9h 3E9h 2E9h  +1     1   DL   Divisor Latch (MSB)  be accessed as word
//!  3FAh 2FAh 3EAh 2EAh  +2     x   IIR  Interrupt Identification Register (r/o) or
//!                                  FCR  FIFO Control Register (w/o, 16550+ only)
//!  3FBh 2FBh 3EBh 2EBh  +3     x   LCR  Line Control Register
//!  3FCh 2FCh 3ECh 2ECh  +4     x   MCR  Modem Control Register
//!  3FDh 2FDh 3EDh 2EDh  +5     x   LSR  Line Status Register
//!  3FEh 2FEh 3EEh 2EEh  +6     x   MSR  Modem Status Register
//!  3FFh 2FFh 3EFh 2EFh  +7     x   SCR  Scratch Register (16450+ and some 8250s,
//!                                      special use with some boards)
//!  
//!            80h      40h      20h      10h      08h      04h      02h      01h
//!  Register  Bit 7    Bit 6    Bit 5    Bit 4    Bit 3    Bit 2    Bit 1    Bit 0
//!  -------------------------------------------------------------------------------
//!  IER         0        0        0        0      EDSSI    ELSI     ETBEI    ERBFI
//!  IIR (r/o) FIFO en  FIFO en    0        0      IID2     IID1     IID0    pending
//!  FCR (w/o)  - RX trigger -     0        0      DMA sel  XFres    RFres   enable
//!  LCR       DLAB     SBR    stick par  even sel Par en  stopbits  - word length -
//!  MCR         0        0        0      Loop     OUT2     OUT1     RTS     DTR
//!  LSR       FIFOerr  TEMT     THRE     Break    FE       PE       OE      RBF
//!  MSR       DCD      RI       DSR      CTS      DDCD     TERI     DDSR    DCTS
//!  
//!  EDSSI:       Enable Delta Status Signals Interrupt
//!  ELSI:        Enable Line Status Interrupt
//!  ETBEI:       Enable Transmitter Buffer Empty Interrupt
//!  ERBFI:       Enable Receiver Buffer Full Interrupt
//!  FIFO en:     FIFO enable
//!  IID#:        Interrupt IDentification
//!  pending:     an interrupt is pending if '0'
//!  RX trigger:  RX FIFO trigger level select
//!  DMA sel:     DMA mode select
//!  XFres:       Transmitter FIFO reset
//!  RFres:       Receiver FIFO reset
//!  DLAB:        Divisor Latch Access Bit
//!  SBR:         Set BReak
//!  stick par:   Stick Parity select
//!  even sel:    Even Parity select
//!  stopbits:    Stop bit select
//!  word length: Word length select
//!  FIFOerr:     At least one error is pending in the RX FIFO chain
//!  TEMT:        Transmitter Empty (last word has been sent)
//!  THRE:        Transmitter Holding Register Empty (new data can be written to THR)
//!  Break:       Broken line detected
//!  FE:          Framing Error
//!  PE:          Parity Error
//!  OE:          Overrun Error
//!  RBF:         Receiver Buffer Full (Data Available)
//!  DCD:         Data Carrier Detect
//!  RI:          Ring Indicator
//!  DSR:         Data Set Ready
//!  CTS:         Clear To Send
//!  DDCD:        Delta Data Carrier Detect
//!  TERI:        Trailing Edge Ring Indicator
//!  DDSR:        Delta Data Set Ready
//!  DCTS:        Delta Clear To Send
//!

#![no_std]

use core::arch::asm;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComPortIo {
    Com1 = 0x3F8,
    Com2 = 0x2F8,
    Com3 = 0x3E8,
    Com4 = 0x2E8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UartKind {
    None,
    Uart8250,
    Uart16450, // 8250 with scratch reg.
    Uart16550,
    Uart16550a,
}

#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BaudDivisor {
    Baud9600 = 0xC,
    Baud19200 = 0x6,
    Baud38400 = 0x3,
    Baud57600 = 0x2,
    Baud115200 = 0x1,
}

#[inline]
#[cfg(target_arch = "x86_64")]
fn outp8(port: u16, val: u8) {
    unsafe {
        asm!("outb %al, %dx", in("al") val, in("dx") port, options(att_syntax, nostack, nomem));
    }
}

#[inline]
#[cfg(target_arch = "x86_64")]
fn outp16(port: u16, val: u16) {
    unsafe {
        asm!("outw %ax, %dx", in("ax") val, in("dx") port, options(att_syntax, nostack, nomem));
    }
}

#[inline]
#[cfg(target_arch = "x86_64")]
fn inp8(port: u16) -> u8 {
    let mut val: u8;

    unsafe {
        asm!("inb %dx, %al", out("al") val, in("dx") port, options(att_syntax, nostack, nomem));
    }
    return val;
}

#[inline]
#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
fn inp16(port: u16) -> u16 {
    let mut val: u16;

    unsafe {
        asm!("inw %dx, %ax", out("ax") val, in("dx") port, options(att_syntax, nostack, nomem));
    }
    return val;
}

fn detect(port: ComPortIo) -> UartKind {
    let data;
    let mut old_data;

    let base_addr = port as u16;
    let iir = base_addr + 2; // Interrupt Id Register:  +2
    let mcr = base_addr + 4; // Modem Control Register: +4
    let msr = base_addr + 6; // Modem Status Register:  +6
    let scr = base_addr + 6; // Scratch register:       +7

    // See if a UART is present anyway

    old_data = inp8(mcr);
    outp8(mcr, 0x10); // Bit 5: Loop

    if inp8(msr) & 0xf0 == 0xf0 {
        // Four MSB bits are set
        return UartKind::None;
    }

    outp8(mcr, 0x1f); // Set Loop=1, OUT2=1, OUT=1, RTS=1, DTR=1
    if (inp8(msr) & 0xf0) != 0xf0 {
        // Must be Loop=1, OUT2=0, OUT=0, RTS=0, DTR=0
        return UartKind::None;
    }

    outp8(mcr, old_data);

    // Now look for the scratch register

    old_data = inp8(scr);
    outp8(scr, 0x55);

    if inp8(scr) != 0x55 {
        return UartKind::Uart8250;
    }

    outp8(scr, 0xaa);
    if inp8(scr) != 0xaa {
        return UartKind::Uart8250;
    }

    outp8(scr, old_data); // We don't need to restore it if it's not there

    // Check if there's a FIFO

    outp8(iir, 1);
    data = inp8(iir);

    // Some old-fashioned software relies on this!

    outp8(iir, 0x0);
    if (data & 0x80) == 0 {
        return UartKind::Uart16450;
    }

    if (data & 0x40) == 0 {
        return UartKind::Uart16550;
    }

    return UartKind::Uart16550a;
}

fn init(port: ComPortIo, kind: UartKind, baud: BaudDivisor) {
    let base_addr = port as u16;
    let ier = base_addr + 1; // Interrupt Enable Register
    let fcr = base_addr + 2; // FIFO Control Register
    let lcr = base_addr + 3; // Line Control Register
    let mcr = base_addr + 4; // Modem Control Register
    let lsr = base_addr + 5; // Line Status Register

    // Access TX/RX 0x00 | 0x03 = 1-stop bit, 8 bit of data, no parity
    outp8(lcr, 0x03);
    // No support for interrupts
    outp8(ier, 0x00);
    // Disable FIFO
    outp8(fcr, 0x00);
    // Reset FIFO if present
    if kind > UartKind::Uart8250 {
        outp8(fcr, 0x06);
    }

    // Access DLAB 0x80 | 0x03 = 1-stop bit, 8 bit of data, no parity
    outp8(lcr, 0x83);

    // Set rate
    outp16(base_addr, baud as u16);

    // Access TX/RX (0x00), 0x03 = 1-stop bit, 8 bit of data, no parity
    outp8(lcr, 0x03);
    // No support for interrupts
    outp8(ier, 0x00);
    if kind > UartKind::Uart8250 {
        // Enable FIFO if present
        outp8(fcr, 0x01);
    }

    outp8(mcr, 0x03); // Ready: DTR | RTS
    outp8(lsr, 0x21); // THRE | RBF
}

fn send_byte(port: ComPortIo, byte: u8) {
    let base_addr = port as u16;
    let lsr = base_addr + 5; // Line Status Register

    // Wait until Transmitter Holding Register is empty
    // (new data can be written to THR)
    while (inp8(lsr) & 0x20) == 0 {
        unsafe { asm!("pause") }
    }

    outp8(base_addr, byte);
}

fn receive_byte(port: ComPortIo) -> u8 {
    let base_addr = port as u16;
    let lsr = base_addr + 5; // Line Status Register

    // Wait until RBF: Receiving buffer full
    // is set.
    // Could also analyze errors cominng from LSR.
    while (inp8(lsr) & 0x1) == 0 {
        unsafe { asm!("pause") }
    }

    return inp8(base_addr);
}

/// Serial portwith 8 bit data, 1 stop bit, and no parity.
pub struct ComPort {
    port: ComPortIo,
    kind: UartKind,
}

impl ComPort {
    pub fn new(port: ComPortIo, baud: BaudDivisor) -> Self {
        let kind = detect(port);
        if kind > UartKind::None {
            init(port, kind, baud);
        }

        Self { port, kind }
    }

    pub fn kind(&self) -> UartKind {
        self.kind
    }

    pub fn send_byte(&self, byte: u8) {
        if self.kind != UartKind::None {
            send_byte(self.port, byte);
        }
    }

    pub fn receive_byte(&self) -> u8 {
        if self.kind != UartKind::None {
            receive_byte(self.port)
        } else {
            0xff
        }
    }
}

impl core::fmt::Write for ComPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.send_byte(byte);
        }
        Ok(())
    }
}
