use qingke::riscv;

pub struct SDIPrint;

impl SDIPrint {
    pub fn enable() {
        unsafe {
            // Enable SDI print
            core::ptr::write_volatile(regs::DEBUG_DATA0_ADDRESS, 0);
            riscv::asm::delay(100000);
        }
    }

    #[inline]
    fn is_busy() -> bool {
        unsafe { core::ptr::read_volatile(regs::DEBUG_DATA0_ADDRESS) != 0 }
    }
}

static mut ENCODER: defmt::Encoder = defmt::Encoder::new();
static mut BUFFER: (usize, [u8; 128]) = (0, [0; 128]);
//static mut CS_RESTORE: critical_section::RestoreState = critical_section::RestoreState::invalid();

fn add_to_write(bytes: &[u8]) {
    unsafe {
        let (len, buffer): &mut (usize, [u8; 128]) = &mut *core::ptr::addr_of_mut!(BUFFER);

        buffer[*len..*len + bytes.len()].clone_from_slice(bytes);
        *len += bytes.len();
    }
}

#[defmt::global_logger]
struct Logger;

mod regs {
    pub const DEBUG_DATA0_ADDRESS: *mut u32 = 0xE000_0380 as *mut u32;
    pub const DEBUG_DATA1_ADDRESS: *mut u32 = 0xE000_0384 as *mut u32;
}

fn do_write(bytes: &[u8]) {
    let mut data = [0u8; 8];
    for chunk in bytes.chunks(7) {
        data[1..chunk.len() + 1].copy_from_slice(chunk);
        data[0] = chunk.len() as u8;

        // data1 is the last 4 bytes of data
        let data1 = u32::from_le_bytes(data[4..].try_into().unwrap());
        let data0 = u32::from_le_bytes(data[..4].try_into().unwrap());

        while SDIPrint::is_busy() {}

        unsafe {
            core::ptr::write_volatile(regs::DEBUG_DATA1_ADDRESS, data1);
            core::ptr::write_volatile(regs::DEBUG_DATA0_ADDRESS, data0);
        }
    }
}

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        // unsafe { CS_RESTORE = critical_section::acquire() };

        unsafe {
            let encoder: &mut defmt::Encoder = &mut *core::ptr::addr_of_mut!(ENCODER);
            encoder.start_frame(add_to_write)
        }
    }

    unsafe fn flush() {}

    unsafe fn release() {
        unsafe {
            let encoder: &mut defmt::Encoder = &mut *core::ptr::addr_of_mut!(ENCODER);
            encoder.end_frame(add_to_write);
            let (len, buffer): &mut (usize, [u8; 128]) = &mut *core::ptr::addr_of_mut!(BUFFER);
            do_write(&buffer[..*len]);
            *len = 0;
        }
        //unsafe {critical_section::release(CS_RESTORE);}
    }

    unsafe fn write(bytes: &[u8]) {
        unsafe {
            let encoder: &mut defmt::Encoder = &mut *core::ptr::addr_of_mut!(ENCODER);
            encoder.write(bytes, add_to_write);
        }
    }
}
