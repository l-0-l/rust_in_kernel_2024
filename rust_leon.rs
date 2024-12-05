//! Rust-based TTY Line Discipline for joystick input

use kernel::{error::to_result, prelude::*};
use core::ffi::{c_char, c_int, c_void, c_uint};
use core::slice;
use core::ptr::null_mut;
use core::mem::MaybeUninit;
use kernel::bindings::{
    kfifo,
    kfifo_alloc,
    kfifo_free,
    kfifo_in,
    kfifo_len,
    kfifo_out,
    kfifo_reset,
    GFP_KERNEL,
    tty_struct,
    input_dev,
    input_report_abs,
    input_report_key,
    input_sync,
    input_allocate_device,
    input_free_device,
    input_register_device,
    input_unregister_device,
    input_set_abs_params,
    input_set_capability,
    EV_ABS, EV_KEY, ABS_X, ABS_Y, BTN_A, BTN_B,
    tty_ldisc_ops,
    tty_register_ldisc,
    tty_unregister_ldisc
};

module! {
    type: TtyInputDriver,
    name: "leon",
    author: "Leon",
    description: "Rust-based TTY HID driver for joystick",
    license: "GPL",
}

// Constant for line discipline
const CUSTOM_LDISC_NUM: c_int = 29; // Just an unused line discipline number

// TtyInputDriver structure
struct TtyInputDriver;

// Structure to hold parsed micro:bit data
struct MicrobitData {
    x: i32, y: i32, z: i32, a: i32, b: i32,
}

impl Default for MicrobitData {
    fn default() -> Self { MicrobitData {x: 0, y: 0, z: 0, a: 0, b: 0} }
}

// Static variable to store the line discipline ops
static mut LDISC_OPS: Option<tty_ldisc_ops> = None;

// Static variable to store the kernel fifo for temporary
// storage of the serial sensor data before processing it
static mut KFIFO: MaybeUninit<kfifo> = MaybeUninit::uninit();

// Static variable to store the Joystick input device struct
static mut INPUT_DEVICE: *mut input_dev = null_mut();

fn setup_input_device(_module: &'static ThisModule) -> Result<()> {
    unsafe {
        // Allocate input device
        let input_dev = input_allocate_device();
        if input_dev.is_null() {
            pr_err!("[LEON] Failed to allocate input device\n");
            return Err(ENOMEM);
        }

        // Set the device name
        (*input_dev).name = b"Leon's Microbit Joystick\0".as_ptr() as *mut c_char;
        (*input_dev).phys = b"Serial line discipline\0".as_ptr() as *mut c_char;

        // Set the device capabilities
        input_set_capability(input_dev, EV_KEY, BTN_A);
        input_set_capability(input_dev, EV_KEY, BTN_B);
        input_set_capability(input_dev, EV_ABS, ABS_X);
        input_set_capability(input_dev, EV_ABS, ABS_Y);

        // Set the absolute axis parameters
        input_set_abs_params(input_dev, ABS_X, -1000, 1000, 100, 0);
        input_set_abs_params(input_dev, ABS_Y, -1000, 1000, 100, 0);

        // Register the input device
        let ret = input_register_device(input_dev);
        if ret != 0 {
            pr_err!("[LEON] Failed to register input device: {}\n", ret);
            input_free_device(input_dev);
            return to_result(ret);
        }

        // Store the input device in static variable
        INPUT_DEVICE = input_dev;
    }

    Ok(())
}

impl kernel::Module for TtyInputDriver {
    fn init(module: &'static ThisModule) -> Result<Self> {
        pr_info!("[LEON] leon module initialized\n");

        // Initialize the kfifo
        unsafe {
            let kfifo_ptr = KFIFO.as_mut_ptr();
            core::ptr::write_bytes(kfifo_ptr as *mut u8, 0, core::mem::size_of::<kfifo>());
            let ret = kfifo_alloc(
                kfifo_ptr,
                BUFFER_SIZE as c_uint,
                GFP_KERNEL,
            );
            if ret != 0 {
                pr_err!("[LEON] Failed to allocate kfifo: {}\n", ret);
                return Err(EINVAL);
            }
        }

        // Register line discipline
        register_line_discipline(module)?;

        // Setup the input device
        setup_input_device(module)?;

        Ok(TtyInputDriver)
    }
}

impl Drop for TtyInputDriver {
    fn drop(&mut self) {
        pr_info!("[LEON] leon module exiting\n");

        // Unregister line discipline
        unregister_line_discipline();

        // Free the kfifo
        unsafe {
            kfifo_free(KFIFO.as_mut_ptr());
        }

        // Unregister and free the input device
        unsafe {
            if !INPUT_DEVICE.is_null() {
                input_unregister_device(INPUT_DEVICE);
                // Note: input_unregister_device() frees the device,
                // so we don't call input_free_device()
                INPUT_DEVICE = null_mut();
            }
        }
    }
}

// Function to register the line discipline
fn register_line_discipline(module: &'static ThisModule) -> Result<()> {
    pr_info!("[LEON] Registering line discipline\n");

    let ops = tty_ldisc_ops {
        name: b"tty_input_ldisc\0".as_ptr() as *mut c_char,
        num: CUSTOM_LDISC_NUM,
        open: Some(tty_ldisc_open),
        close: Some(tty_ldisc_close),
        flush_buffer: Some(tty_ldisc_flush_buffer),
        receive_buf2: Some(tty_ldisc_receive_buf2),
        owner: module.as_ptr(),
        ..Default::default()
    };

    unsafe {
        // Store the ops in the static variable
        LDISC_OPS = Some(ops);

        // Get a mutable pointer to the stored ops
        let ops_ptr = LDISC_OPS.as_mut().unwrap() as *mut tty_ldisc_ops;

        // Register the line discipline
        let ret = tty_register_ldisc(ops_ptr);
        if ret != 0 {
            pr_err!("[LEON] Failed to register line discipline: {}\n", ret);
            LDISC_OPS = None;
            return to_result(ret);
        }
    }

    pr_info!("[LEON] Successfully registered line discipline\n");

    Ok(())
}

// Function to unregister the line discipline
fn unregister_line_discipline() {
    pr_info!("[LEON] Unregistering line discipline\n");

    unsafe {
        if let Some(ref mut ops) = LDISC_OPS {
            let ops_ptr = ops as *mut tty_ldisc_ops;
            tty_unregister_ldisc(ops_ptr);
            LDISC_OPS = None;
            pr_info!("[LEON] Successfully unregistered line discipline\n");
        } else {
            pr_err!("[LEON] No line discipline to unregister\n");
        }
    }
}

// Line discipline open callback
unsafe extern "C" fn tty_ldisc_open(tty: *mut tty_struct) -> c_int {
    // Check if tty is not null
    if tty.is_null() {
        pr_err!("[LEON] tty_ldisc_open: tty is null\n");
        return EINVAL.to_errno();
    }

    // Convert raw pointer to a mutable reference safely
    let tty_ref = unsafe { &mut *tty };

    // Set receive_room
    tty_ref.receive_room = 65535;

    unsafe { kfifo_reset(KFIFO.as_mut_ptr()); }

    pr_info!("[LEON] tty_ldisc_open: Line discipline open\n");

    0 // Return success
}

// Line discipline close callback
unsafe extern "C" fn tty_ldisc_close(_tty: *mut tty_struct) {
    pr_info!("[LEON] tty_ldisc_close: Line discipline close\n");
}

// Line discipline flush buffer callback, called before close
unsafe extern "C" fn tty_ldisc_flush_buffer(_tty: *mut tty_struct) {
    pr_info!("[LEON] flush_buffer called\n");
}

const BUFFER_SIZE: usize = 1024;

unsafe extern "C" fn tty_ldisc_receive_buf2(
    _tty: *mut tty_struct,
    cp: *const u8,
    _fp: *const u8,
    count: usize,
) -> usize {
    // Unsafe operation: creating a slice from raw parts
    let input_data = unsafe { slice::from_raw_parts(cp, count) };

    // Check if the input_data contains a '\n'
    if let Some(pos) = input_data.iter().position(|&c| c == b'\n') {
        // We have a '\n' in the current input_data

        // Unsafe operation: accessing KFIFO
        let fifo_len = unsafe { kfifo_len(KFIFO.as_mut_ptr()) } as usize;
        let mut temp_buffer = [0u8; BUFFER_SIZE];

        let mut total_len = 0usize;

        if fifo_len > 0 {
            // Limit the read length to prevent overflow
            let read_len = core::cmp::min(fifo_len, BUFFER_SIZE - total_len);
            // Unsafe operation: reading from KFIFO into temp_buffer
            let read_bytes = unsafe {
                kfifo_out(
                    KFIFO.as_mut_ptr(),
                    temp_buffer.as_mut_ptr() as *mut c_void,
                    read_len as c_uint,
                )
            };
            if read_bytes != read_len as c_uint {
                pr_err!("[LEON] Failed to read from kfifo\n");
                // Handle the error as needed
            }
            total_len += read_bytes as usize;
        }

        // Ensure we don't overflow temp_buffer
        let copy_len = core::cmp::min(pos + 1, BUFFER_SIZE - total_len);

        // Append data from input_data up to and including '\n'
        temp_buffer[total_len..total_len + copy_len]
            .copy_from_slice(&input_data[..copy_len]);
        total_len += copy_len;

        // Convert temp_buffer to str and process it
        if let Ok(line_str) = core::str::from_utf8(&temp_buffer[..total_len]) {
            if let Err(err) = parse_microbit_data(line_str) {
                pr_err!("[LEON] parse_microbit_data error: {}\n", err);
                // Unsafe operation: resetting KFIFO
                unsafe { kfifo_reset(KFIFO.as_mut_ptr()) };
            }
        } else {
            pr_err!("[LEON] Received non-UTF8 data\n");
            // Unsafe operation: resetting KFIFO
            unsafe { kfifo_reset(KFIFO.as_mut_ptr()) };
        }

        // If there is remaining data in input_data after '\n', push it into KFIFO
        if pos + 1 < input_data.len() {
            let remaining_data = &input_data[pos + 1..];
            // Unsafe operation: writing to KFIFO
            let bytes_written = unsafe {
                kfifo_in(
                    KFIFO.as_mut_ptr(),
                    remaining_data.as_ptr() as *const c_void,
                    remaining_data.len() as c_uint,
                )
            };
            if bytes_written != remaining_data.len() as c_uint {
                if let Ok(remaining_str) = core::str::from_utf8(remaining_data) {
                    pr_err!(
                        "[LEON] kfifo overflow, wrote {} bytes, tried to write {} bytes of '{}'\n",
                        bytes_written,
                        remaining_data.len(),
                        remaining_str
                    );
                }
            }
        }
    } else {
        // No '\n' in input_data, push all data into KFIFO
        // Unsafe operation: writing to KFIFO
        let bytes_written = unsafe {
            kfifo_in(
                KFIFO.as_mut_ptr(),
                input_data.as_ptr() as *const c_void,
                input_data.len() as c_uint,
            )
        };

        if bytes_written != input_data.len() as c_uint {
            pr_err!(
                "[LEON] kfifo overflow, wrote {} bytes, tried to write {} bytes\n",
                bytes_written,
                input_data.len()
            );
        }
    }

    count // Return the number of bytes consumed
}

// Function to parse the received data into MicrobitData
fn parse_microbit_data(line: &str)  -> Result<(), &'static str> {
    // Expected format: "X:{},Y:{},Z:{},A:{},B:{}\n"
    let line = line.trim_end_matches(|c| c == '\n' || c == '\r');

    // Struct to hold parsed data
    let mut data = MicrobitData::default();

    // Helper function to parse key-value pairs
    fn parse_key_value(key: &str, value: &str, data: &mut MicrobitData) -> Result<(), &'static str> {
        let parsed_value = value.parse::<i32>().map_err(|_| "Invalid value")?;
        match key {
            "X" => data.x = parsed_value,
            "Y" => data.y = parsed_value,
            "Z" => data.z = parsed_value,
            "A" => data.a = parsed_value,
            "B" => data.b = parsed_value,
            _ => return Err("Unknown key"),
        }
        Ok(())
    }

    // Attempt to parse each part
    for part in line.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = match kv.next() {
            Some(k) => k,
            None => {
                pr_err!("[LEON] Missing key in part '{}'\n", part);
                return Err("Missing key");
            }
        };
        let value = match kv.next() {
            Some(v) => v,
            None => {
                pr_err!("[LEON] Missing value for key '{}'\n", key);
                return Err("Missing value");
            }
        };
        if let Err(err) = parse_key_value(key, value, &mut data) {
            pr_err!("[LEON] Error parsing {}: {}\n", key, err);
            return Err(err);
        }
    }

    // Print the parsed data if all parts are valid
    // pr_info!(
    //     "[LEON] Parsed data: X:{}, Y:{}, Z:{}, A:{}, B:{}\n",
    //     data.x, data.y, data.z, data.a, data.b
    // );

    // Use the parsed data, i.e., just push it as is into the preconfigured input device
    unsafe {
        if !INPUT_DEVICE.is_null() {
            input_report_abs(INPUT_DEVICE, ABS_X, data.x);
            input_report_abs(INPUT_DEVICE, ABS_Y, data.y);
            input_report_key(INPUT_DEVICE, BTN_A, data.a);
            input_report_key(INPUT_DEVICE, BTN_B, data.b);
            input_sync(INPUT_DEVICE);
        } else {
            pr_err!("[LEON] Input device is null, have you gone mad?!\n");
            return Err("Input device is null");
        }
    }

    Ok(())
}
