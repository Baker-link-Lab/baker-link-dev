#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

#[rtic::app(device = rp_pico::hal::pac, dispatchers = [TIMER_IRQ_1])]
mod app {
    use embedded_hal::digital::v2::OutputPin;
    use panic_halt as _;
    use rp_pico as _;
    use rp_pico::hal::{
        gpio::bank0::{Gpio12, Gpio14, Gpio2, Gpio10, Gpio11, Gpio13},
        usb::UsbBus,
    };
    use usb_device::{
        class_prelude::UsbBusAllocator,
        prelude::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };
    use usbd_serial::SerialPort;

    // DIO:GPIO14, CLK:GPIO12, LED:GPIO0(Green), GPIO2(RED), Button:GPIO1
    // DIO:GPIO12, CLK:GPIO13, LED:GPIO0(Green), GPIO2(RED), Button:GPIO1
    type GpioSwDio = Gpio14;
    type GpioSwClk = Gpio12;
    // type GpioSwDio = Gpio12;
    // type GpioSwClk = Gpio13;


    type PowerLed = Gpio2;

    type SwdIoPins = rust_dap_rp2040::util::SwdIoSet<GpioSwClk, GpioSwDio>;

    #[shared]
    struct Shared {
        usb_serial: SerialPort<'static, UsbBus>,
    }

    #[local]
    struct Local {
        usb_bus: UsbDevice<'static, UsbBus>,
        usb_dap: rust_dap::CmsisDap<'static, UsbBus, SwdIoPins, 64>,
        power_led: rp_pico::hal::gpio::Pin<
            PowerLed,
            rp_pico::hal::gpio::Output<rp_pico::hal::gpio::PushPull>,
        >,
    }

    #[init(local=[USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None])]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut resets = ctx.device.RESETS;

        let mut watchdog = rp_pico::hal::Watchdog::new(ctx.device.WATCHDOG);
        let clocks = rp_pico::hal::clocks::init_clocks_and_plls(
            rp_pico::XOSC_CRYSTAL_FREQ,
            ctx.device.XOSC,
            ctx.device.CLOCKS,
            ctx.device.PLL_SYS,
            ctx.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = rp_pico::hal::Sio::new(ctx.device.SIO);
        let pins = rp_pico::Pins::new(
            ctx.device.IO_BANK0,
            ctx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        let mut swdio_pin = pins.gpio14.into_mode();
        let mut swclk_pin = pins.gpio12.into_mode();
        swdio_pin.set_slew_rate(rp_pico::hal::gpio::OutputSlewRate::Fast);
        swclk_pin.set_slew_rate(rp_pico::hal::gpio::OutputSlewRate::Fast);
        let swdio_pins = SwdIoPins::new(ctx.device.PIO0, swclk_pin, swdio_pin, &mut resets);

        let usb_allocator = UsbBusAllocator::new(rp_pico::hal::usb::UsbBus::new(
            ctx.device.USBCTRL_REGS,
            ctx.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        ));
        ctx.local.USB_ALLOCATOR.replace(usb_allocator);
        let usb_allocator = ctx.local.USB_ALLOCATOR.as_ref().unwrap();

        let usb_serial = usbd_serial::SerialPort::new(&usb_allocator);
        let usb_dap =
            rust_dap::CmsisDap::new(&usb_allocator, swdio_pins, rust_dap::DapCapabilities::SWD);
        let usb_bus = UsbDeviceBuilder::new(&usb_allocator, UsbVidPid(0x2E8A, 0x106B))
            .manufacturer("Seebeck inc.")
            .product("Baker link. Dev(CMSIS-DAP)")
            // .serial_number("")
            .device_class(rust_dap::USB_CLASS_MISCELLANEOUS)
            .device_class(rust_dap::USB_SUBCLASS_COMMON)
            .device_protocol(rust_dap::USB_PROTOCOL_IAD)
            .composite_with_iads()
            .max_packet_size_0(64)
            .build();

        let power_led = pins.gpio2.into_push_pull_output();

        (
            Shared { usb_serial },
            Local {
                usb_bus,
                usb_dap,
                power_led,
            },
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle(local = [power_led])]
    fn idle(ctx: idle::Context) -> ! {
        ctx.local.power_led.set_high().unwrap();
        loop {}
    }

    #[task(binds = USBCTRL_IRQ, priority = 1, shared = [usb_serial], local = [usb_bus, usb_dap])]
    fn dap_run(mut ctx: dap_run::Context) {
        let poll_result = ctx
            .shared
            .usb_serial
            .lock(|usb_serial| ctx.local.usb_bus.poll(&mut [usb_serial, ctx.local.usb_dap]));
        if !poll_result {
            return;
        }
        match ctx.local.usb_dap.process() {
            Ok(_) => {
                // ctx.local.run_led.set_low().unwrap();
            }
            Err(_) => {}
        }
    }
}
