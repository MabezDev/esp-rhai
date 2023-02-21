#![no_std]
#![no_main]

extern crate alloc;
use core::fmt::Write;
use esp32c3_hal::{
    clock::ClockControl,
    peripherals::Peripherals,
    prelude::{nb::block, *},
    timer::TimerGroup,
    Rtc,
};
use esp_backtrace as _;
use esp_println::{print, println};
use heapless::String;
use rhai::{Engine, INT, packages::{BasicStringPackage, Package}};

#[cfg(feature = "uart0")]
use esp32c3_hal::Uart;
#[cfg(feature = "usb-serial-jtag")]
use esp32c3_hal::UsbSerialJtag;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 64 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }

    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    init_heap();

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    #[cfg(feature = "usb-serial-jtag")]
    let mut x = UsbSerialJtag::new(peripherals.USB_DEVICE);
    #[cfg(feature = "uart0")]
    let mut x = Uart::new(peripherals.UART0);

    let mut buffer = String::<1024>::new();

    let mut engine = Engine::new_raw();
    let bsp = BasicStringPackage::new();
    bsp.register_into_engine(&mut engine); // has print and debug

    engine.register_fn("heap", heap);
    engine.register_fn("heap_stats", heap_stats);
    engine.on_debug(move |s, src, pos| {
        let src = src.unwrap_or("unknown");
        println!("DEBUG of {src} at {pos:?}: {s}");
    });
    engine.on_print(|s: &str| { println!("{s}") });

    // run abitrary scripts
    println!("Running example script...");
    engine.run(r#"
        print("hello, world!");
        let x = 12;
        let y = 44;
        let result = x * y;
        print(`${x} * ${y} = ${result}`);
    "#).unwrap();

    write!(x, "\nesp-rhai repl - v{}\n>>> ", env!("CARGO_PKG_VERSION")).ok();
    block!(x.flush()).unwrap();
    loop {
        let c = block!(x.read()).unwrap();
        block!(x.write(c)).unwrap();

        let mut execute = false;
        if c == 8 {
            buffer.pop(); // delete last
        } else if c == b'\r' {
            if buffer.len() > 0 {
                execute = true; // enter
            } else {
                write!(x, "\n>>> ").unwrap();
            }
        } else {
            buffer.push(c as char).unwrap();
        }

        if execute {
            match buffer.as_bytes() {
                [b'c', b'a', b'l', b'c', ..] => match engine.eval_expression::<INT>(&buffer[4..]) {
                    Ok(res) => write!(x, "\n{res}\n").unwrap(),
                    Err(e) => writeln!(x, "\n{e:?}\n").unwrap(),
                },
                _ => {
                    writeln!(x).unwrap();
                    match engine.eval_expression::<()>(&buffer[..]) {
                        Ok(_) => {}
                        Err(e) => writeln!(x, "{e:?}\n").unwrap(),
                    }
                }
            }
            print!(">>> ");
            buffer.clear();
        }

        block!(x.flush()).unwrap();
    }
}

fn heap() {
    println!("used = {}, free = {}", ALLOCATOR.used(), ALLOCATOR.free())
}

fn heap_stats() -> (usize, usize) {
    (ALLOCATOR.used(), ALLOCATOR.free())
}

#[no_mangle]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}
