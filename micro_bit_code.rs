#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use embedded_io::Write;
use panic_rtt_target as _; // For a panic_handler function
use rtt_target::{rprintln, rtt_init_print};

use lsm303agr::{AccelOutputDataRate, Lsm303agr};

use microbit::{
    board::Board,
    display::blocking::Display,
    hal::{twi, uart::{self, Baudrate, Parity}, Timer},
    pac::twi0::frequency::FREQUENCY_A
};

/// The 5x5 LED matrix patterns to display.
const MATRIX: [[[u8; 5]; 5]; 12] = [
    [[0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0]],
    [[0, 0, 0, 1, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 1, 0, 0, 0]],
    [[0, 0, 0, 1, 0], [0, 0, 0, 1, 0], [0, 0, 1, 0, 0], [0, 1, 0, 0, 0], [0, 1, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [0, 0, 0, 1, 0], [0, 0, 1, 0, 0], [0, 1, 0, 0, 0], [0, 0, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [0, 0, 0, 1, 1], [0, 0, 1, 0, 0], [1, 1, 0, 0, 0], [0, 0, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [0, 0, 0, 0, 1], [0, 1, 1, 1, 0], [1, 0, 0, 0, 0], [0, 0, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [0, 0, 0, 0, 0], [1, 1, 1, 1, 1], [0, 0, 0, 0, 0], [0, 0, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [1, 0, 0, 0, 0], [0, 1, 1, 1, 0], [0, 0, 0, 0, 1], [0, 0, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [1, 1, 0, 0, 0], [0, 0, 1, 0, 0], [0, 0, 0, 1, 1], [0, 0, 0, 0, 0]],
    [[0, 0, 0, 0, 0], [0, 1, 0, 0, 0], [0, 0, 1, 0, 0], [0, 0, 0, 1, 0], [0, 0, 0, 0, 0]],
    [[0, 1, 0, 0, 0], [0, 1, 0, 0, 0], [0, 0, 1, 0, 0], [0, 0, 0, 1, 0], [0, 0, 0, 1, 0]],
    [[0, 1, 0, 0, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 0, 1, 0, 0], [0, 0, 0, 1, 0]]
];

/// The entry point of the application.
///
/// Initializes the RTT (Real-Time Transfer) for printing, sets up the UART for serial communication,
/// and configures the I2C interface for the LSM303AGR accelerometer. It collects and displays the
/// accelerometer data via both serial and RTT.
#[entry]
fn main() -> ! {
    rtt_init_print!();

    // Initialize the board and peripherals
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);

    // Set up the UART for serial communication with 1200 baud rate and no parity
    let mut serial = uart::Uart::new(
        board.UART0,
        board.uart.into(),
        Parity::EXCLUDED,
        Baudrate::BAUD115200,
    );

    // Configure the button inputs with pull-up resistors
    let mut button_a = board.buttons.button_a.into_pullup_input();
    let mut button_b = board.buttons.button_b.into_pullup_input();

    // Set up the I2C interface for the LSM303AGR accelerometer
    let i2c = twi::Twi::new(board.TWI0, board.i2c.into(), FREQUENCY_A::K100);
    let mut sensor = Lsm303agr::new_with_i2c(i2c);

    // Initialize the accelerometer and set its mode and output data rate
    sensor.init().unwrap();
    sensor
        .set_accel_mode_and_odr(
            &mut timer,
            lsm303agr::AccelMode::HighResolution,
            AccelOutputDataRate::Khz1_344,
        )
        .unwrap();

    let mut iterator = MATRIX.iter().cycle();
    let mut display = Display::new(board.display_pins);

    loop {

        display.show(&mut timer, *iterator.next().unwrap(), 25);

        // Read acceleration data from the sensor
        let data = sensor.acceleration().unwrap();

        // Read the state of the buttons
        let buttons_state = (button_a.is_low().unwrap(), button_b.is_low().unwrap());

        // Write the acceleration data to the serial output
        write!(
            serial,
            "X:{},Y:{},Z:{},A:{},B:{}\n",
            data.x_unscaled(),
            data.y_unscaled(),
            data.z_unscaled(),
            buttons_state.0 as u8,
            buttons_state.1 as u8
        )
        .unwrap();

        // Print the acceleration data to the RTT output
        rprintln!(
            "X: {:>5}, Y: {:>5}, Z: {:>5}, A: {}, B: {}",
            data.x_unscaled(),
            data.y_unscaled(),
            data.z_unscaled(),
            buttons_state.0 as u8,
            buttons_state.1 as u8
        );
    }
}
