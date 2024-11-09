#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

#[rtic::app(device = rp_pico::hal::pac, dispatchers = [TIMER_IRQ_1])]
mod app {
    use embedded_hal::digital::v2::{InputPin, OutputPin};
    use panic_halt as _;
    use rp_pico as _;
    use rp_pico::hal::{
        gpio::bank0::{Gpio0, Gpio10, Gpio11, Gpio12, Gpio14, Gpio2},
        usb::UsbBus,
    };
    use usb_device::{
        class_prelude::UsbBusAllocator,
        prelude::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };
    use usbd_serial::SerialPort;

    // DIO:GPIO12, CLK:GPIO13, LED:GPIO0(Green), GPIO2(RED), Button:GPIO1
    type GpioSwDio1 = Gpio14;
    type GpioSwClk1 = Gpio12;

    type GpioSwDio2 = Gpio10;
    type GpioSwClk2 = Gpio11;

    type GreenLed = Gpio0;
    type PowerLed = Gpio2;

    type SwdIoPins1 = rust_dap_rp2040::util::SwdIoSet<GpioSwClk1, GpioSwDio1>;
    type SwdIoPins2 = rust_dap_rp2040::util::SwdIoSet<GpioSwClk2, GpioSwDio2>;

    enum DapType {
        Dap1(rust_dap::CmsisDap<'static, UsbBus, SwdIoPins1, 64>),
        Dap2(rust_dap::CmsisDap<'static, UsbBus, SwdIoPins2, 64>),
    }

    #[shared]
    struct Shared {
        usb_serial: SerialPort<'static, UsbBus>,
    }

    #[local]
    struct Local {
        usb_bus: UsbDevice<'static, UsbBus>,
        usb_dap: DapType,
        power_led: rp_pico::hal::gpio::Pin<
            PowerLed,
            rp_pico::hal::gpio::Output<rp_pico::hal::gpio::PushPull>,
        >,
        green_led: rp_pico::hal::gpio::Pin<
            GreenLed,
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

        let button = pins.gpio18.into_pull_up_input();

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

        let power_led = pins.gpio2.into_push_pull_output();
        let green_led = pins.gpio0.into_push_pull_output();

        let mut cnt = 0;
        for _ in 0..5 {
            if button.is_low().unwrap() {
                cnt += 1;
            }
        }
        let usb_dap;
        if cnt >= 3 {
            let mut swdio_pin = pins.gpio10.into_mode();
            let mut swclk_pin = pins.gpio11.into_mode();
            swdio_pin.set_slew_rate(rp_pico::hal::gpio::OutputSlewRate::Fast);
            swclk_pin.set_slew_rate(rp_pico::hal::gpio::OutputSlewRate::Fast);
            let swdio_pins = SwdIoPins2::new(ctx.device.PIO0, swclk_pin, swdio_pin, &mut resets);
            usb_dap = DapType::Dap2(rust_dap::CmsisDap::new(
                &usb_allocator,
                swdio_pins,
                rust_dap::DapCapabilities::SWD,
            ));
        } else {
            let mut swdio_pin = pins.gpio14.into_mode();
            let mut swclk_pin = pins.gpio12.into_mode();
            swdio_pin.set_slew_rate(rp_pico::hal::gpio::OutputSlewRate::Fast);
            swclk_pin.set_slew_rate(rp_pico::hal::gpio::OutputSlewRate::Fast);
            let swdio_pins = SwdIoPins1::new(ctx.device.PIO0, swclk_pin, swdio_pin, &mut resets);
            usb_dap = DapType::Dap1(rust_dap::CmsisDap::new(
                &usb_allocator,
                swdio_pins,
                rust_dap::DapCapabilities::SWD,
            ));
        }
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

        (
            Shared { usb_serial },
            Local {
                usb_bus: usb_bus,
                usb_dap: usb_dap,
                power_led: power_led,
                green_led: green_led,
            },
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle(local = [power_led])]
    fn idle(ctx: idle::Context) -> ! {
        ctx.local.power_led.set_high().unwrap();
        loop {}
    }

    #[task(binds = USBCTRL_IRQ, priority = 1, shared = [usb_serial], local = [usb_bus, usb_dap, green_led])]
    fn dap_run(mut ctx: dap_run::Context) {
        let green_led = ctx.local.green_led;
        match ctx.local.usb_dap {
            DapType::Dap1(usb_dap) => {
                let poll_result = ctx
                    .shared
                    .usb_serial
                    .lock(|usb_serial| ctx.local.usb_bus.poll(&mut [usb_serial, usb_dap]));
                if !poll_result {
                    return;
                }
                match usb_dap.process() {
                    Ok(_) => {
                        // ctx.local.run_led.set_low().unwrap();
                    }
                    Err(_) => {}
                }
            }
            DapType::Dap2(usb_dap) => {
                let poll_result = ctx
                    .shared
                    .usb_serial
                    .lock(|usb_serial| ctx.local.usb_bus.poll(&mut [usb_serial, usb_dap]));
                green_led.set_high().unwrap();
                if !poll_result {
                    return;
                }
                match usb_dap.process() {
                    Ok(_) => {
                        // ctx.local.run_led.set_low().unwrap();
                    }
                    Err(_) => {}
                }
            }
        }
    }
}
