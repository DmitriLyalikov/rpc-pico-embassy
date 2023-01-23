#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

/// RP2040 RPC Server for Interface Bridging Application
/// Author: Dmitri Lyalikov 
/// Version: 0.1.0
/// 
/// This is the binary application that will run on the RP2040 chip. It will listen for 'RPC' requests from a SPI master.
/// An 'RPC' request is a multi byte message that specifies the request-id, interface, payload, and CRC-8. 
/// This request will be received and payload data will be written to the TX FIFO to the PIO state machine that implements 
/// the desired interface (SMI, SPI, JTAG, I2C, etc..)

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Input, Pull, Pin};
use embassy_rp::pac::resets;
use embassy_rp::pio::{Pio0, PioPeripherial, PioStateMachine, PioStateMachineInstance, Sm0, Sm1};
use embassy_rp::relocate::RelocatedProgram;
use embassy_rp::spi::{Config, Spi};
use embassy_rp::reset;
//use embassy_rp::peripherals::SPI1;
use embassy_rp::pac::resets::regs::Peripherals;
use embassy_rp::{pio_instr_util};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_rp::interrupt;

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn pio_task_blink(mut sm: PioStateMachineInstance<Pio0, Sm1>, pin: AnyPin) {
    // Simple Blink PIO
    let prg = pio_proc::pio_asm!(
        ".origin 0",
        "set pindirs,1",
        ".wrap_target",
        "set pins,0 [31]",
        "set pins,0 [31]",
        "set pins,0 [31]",
        "set pins,0 [31]",
        "set pins,1 [31]",
        "set pins,1 [31]",
        "set pins,1 [31]",
        "set pins,1 [31]",
        ".wrap",
    );
    let relocated = RelocatedProgram::new(&prg.program);
    let out_pin = sm.make_pio_pin(pin);
    let pio_pins = [&out_pin];
    sm.set_set_pins(&pio_pins);
    sm.set_set_range(25, 1);

    sm.write_instr(relocated.origin() as usize, relocated.code());
    pio_instr_util::exec_jmp(&mut sm, relocated.origin());
    // Set clock to ~1KHz
    sm.set_clkdiv((125e6 / 20.0 / 2e2 * 256.0) as u32);

    let pio::Wrap { source, target } = relocated.wrap();
    sm.set_wrap(source, target);

    sm.set_enable(true);
    info!("Loaded PIO blink program");
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialise Peripherals
    let p = embassy_rp::init(Default::default());

    
    let miso = p.PIN_12;
    let mosi = p.PIN_11;
    let clk = p.PIN_10;
    let cs = p.PIN_13;
    let mut async_input = Input::new(cs, Pull::Down);

    // Configure and Enable our SPI Slave
  
    let mut config = Config::default();
    config.slave = true;
    let mut spi = Spi::new(p.SPI1, clk, mosi, miso, p.DMA_CH0, p.DMA_CH1, config);
    //let mut spi = Spi::new(p.SPI1, clk, mosi, miso, config);
    spi.set_slave(true);
    
    let pio = p.PIO0;

    let (_, _sm0, sm1, ..) = pio.split();
    spawner.spawn(pio_task_blink(sm1, p.PIN_25.degrade())).unwrap();
    let irq = interrupt::take!(USBCTRL_IRQ);
    let driver = Driver::new(p.USB, irq);
    spawner.spawn(logger_task(driver)).unwrap();

    loop {
        let tx_buf = [1_u8, 2, 3, 4, 5, 6];
        let mut rx_buf = [0_u8; 6];
        async_input.wait_for_low().await;
        spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
        info!("{:?}", rx_buf);
        Timer::after(Duration::from_secs(1)).await; 
    }
}

