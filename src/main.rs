#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Level, Output, Pin};
use embassy_rp::pio::{Pio0, PioPeripherial, PioStateMachine, PioStateMachineInstance, Sm0, Sm1};
use embassy_rp::relocate::RelocatedProgram;
use embassy_rp::spi::{Blocking, Spi};
use embassy_rp::{pio_instr_util, spi};
use embassy_time::{Delay, Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

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
    sm.set_clkdiv(0);
    // sm.set_clkdiv((125e6 / 20.0 / 2e2 * 256.0) as u32);

    let pio::Wrap { source, target } = relocated.wrap();
    sm.set_wrap(source, target);

    //     sm.set_clkdiv((125e6 / 20.0 / 2e2 * 256.0) as u32);
    sm.set_enable(true);
    info!("started");
}



#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialise Peripherals
    let p = embassy_rp::init(Default::default());
    let pio = p.PIO0;

    let (_, sm0, sm1, ..) = pio.split();
    spawner.spawn(pio_task_blink(sm1, p.PIN_25.degrade())).unwrap();

    // Create LED
    // let mut led = Output::new(p.PIN_25, Level::Low);

    // Loop
    loop {
        // Log
        info!("LED On!");

        // Turn LED On
       // led.set_high();

        // Wait 100ms
        Timer::after(Duration::from_millis(100)).await;

        // Log
        info!("LED Off!");

        // Turn Led Off
       // led.set_low();

        // Wait 100ms
        Timer::after(Duration::from_millis(100)).await;
    }
}

