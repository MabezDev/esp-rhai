#![no_std]
#![no_main]

extern crate alloc;
use esp32c3_hal::{
    clock::ClockControl, peripherals::Peripherals, prelude::{*, nb::block}, timer::TimerGroup, Rtc, UsbSerialJtag,
};
use esp_backtrace as _;
use esp_println::println;
use heapless::String;
use rhai::{Engine, INT};

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

    let mut x = UsbSerialJtag::new(peripherals.USB_DEVICE);

    let mut buffer = String::<1024>::new();

    let mut engine = Engine::new_raw();
    engine.register_fn("heap", heap);
    engine.register_fn("print", print);
    engine.register_fn("debug", debug); // this doesn't seem to be possible sadly :(
    engine.on_print(|s: &str| -> () { println!("{s}") });
    // run abitrary scripts
    engine.run(r#"
        print("hello, world!");
        let x = 12;
        let y = 44;
        let result = x * y;
        print(`${x} * ${y} = ${result}`);
    "#).unwrap();

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
                println!(">>> ");
            }
        } else {
            buffer.push(c as char).unwrap();
        }
        
        if execute {
            println!("Command: {}", buffer);
            match buffer.as_bytes() {
                [b'c',b'a',b'l',b'c', ..] => match engine.eval_expression::<INT>(&buffer[4..]) {
                    Ok(res) => println!(">>> {}", res),
                    Err(e) => println!("{:?}", e)
                },
                _ => match engine.eval_expression::<()>(&buffer[..]) {
                    Ok(_) => println!(">>> "),
                    Err(e) => println!("{:?}", e)
                }
            }
            buffer.clear();
        }
    }
}

fn heap() {
    println!("used = {}, free = {}", ALLOCATOR.used(), ALLOCATOR.free())
}

fn print(s: &str) -> &str {
    s
}

fn debug(d: &dyn core::fmt::Debug) {
    println!("{:?}", d);
}


#[no_mangle]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}
