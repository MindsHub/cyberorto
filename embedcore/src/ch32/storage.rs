use ch32_hal::pac::flash::Flash;
use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash, check_erase, check_read,
    check_write,
};

pub struct FlashTest {
    data: &'static mut [u32],
    flash: Flash,
}
#[derive(Debug)]
pub enum FlashErrors {
    FlashAdrRangeError,
    NotAligned,
    Other,
}
impl Default for FlashTest {
    fn default() -> Self {
        unsafe { Self::new(0x08030000 as *mut u32, 256 * 4) }
    }
}
impl FlashTest {
    ///
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer.
    unsafe fn new(start: *mut u32, size: usize) -> Self {
        let data = unsafe { core::slice::from_raw_parts_mut::<u32>(start, size) };
        let a = 0x40000000 + 0x20000 + 0x2000;
        let flash = unsafe { Flash::from_ptr(a as *mut ()) };

        Self { data, flash }
    }

    const FLASH_KEY1: u32 = 0x45670123;
    const FLASH_KEY2: u32 = 0xCDEF89AB;
}

impl NorFlashError for FlashErrors {
    fn kind(&self) -> NorFlashErrorKind {
        match &self {
            FlashErrors::FlashAdrRangeError => NorFlashErrorKind::OutOfBounds,
            FlashErrors::NotAligned => NorFlashErrorKind::NotAligned,
            FlashErrors::Other => NorFlashErrorKind::Other,
        }
    }
}
impl From<NorFlashErrorKind> for FlashErrors {
    fn from(value: NorFlashErrorKind) -> Self {
        match value {
            NorFlashErrorKind::NotAligned => Self::NotAligned,
            NorFlashErrorKind::OutOfBounds => Self::FlashAdrRangeError,
            NorFlashErrorKind::Other => Self::Other,
            _ => Self::Other,
        }
    }
}

impl ErrorType for FlashTest {
    type Error = FlashErrors;
}

impl ReadNorFlash for FlashTest {
    const READ_SIZE: usize = 4;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        check_read(&self, offset, bytes.len())?;
        let start = offset as usize;
        let end = start + bytes.len();
        let chunk = bytes.chunks_exact_mut(4);
        for (from, into) in self.data[start / Self::READ_SIZE..end / Self::READ_SIZE]
            .iter()
            .zip(chunk)
        {
            let cur = (from).to_ne_bytes();
            into.copy_from_slice(&cur);
        }
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.data.len()
    }
}

impl NorFlash for FlashTest {
    const WRITE_SIZE: usize = 256;

    const ERASE_SIZE: usize = 256;

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        check_write(self, offset, bytes.len())?;
        let offset = offset as usize;

        let n_integer = bytes.len() / 4;
        let from = bytes.chunks_exact(256).map(|x| {
            let mut data = [0u32; 64];
            let iterator = x
                .chunks_exact(4)
                .map(|x| {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(x);
                    u32::from_ne_bytes(buf)
                })
                .enumerate();
            for (index, val) in iterator {
                data[index] = val;
            }
            data
        });

        // Authorize the FPEC of Bank1 Access
        self.flash.keyr().write(|x| x.set_keyr(Self::FLASH_KEY1));
        self.flash.keyr().write(|x| x.set_keyr(Self::FLASH_KEY2));

        // Fast program mode unlock
        self.flash
            .modekeyr()
            .write(|x| x.set_modekeyr(Self::FLASH_KEY1));
        self.flash
            .modekeyr()
            .write(|x| x.set_modekeyr(Self::FLASH_KEY2));

        // prepare data
        let into = self.data[offset / 4..offset / 4 + n_integer].chunks_exact_mut(64);
        //actual chunked write
        for (into, from) in into.zip(from) {
            //we write chunks of u32 in order to accelerate the write
            self.flash.ctlr().modify(|x| x.set_page_pg(true));
            while self.flash.statr().read().bsy() {}
            while self.flash.statr().read().wr_bsy() {}

            for (i, f) in into.iter_mut().zip(&from) {
                *i = *f;
                //wait
                while self.flash.statr().read().wr_bsy() {}
            }
            //actual write
            self.flash.ctlr().modify(|x| x.set_pgstart(true));
            while self.flash.statr().read().bsy() {}
            self.flash.ctlr().modify(|x| x.set_page_pg(false));
        }
        self.flash.ctlr().modify(|x| x.set_flock(true));
        self.flash.ctlr().modify(|x| x.set_lock(true));

        Ok(())
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        check_erase(self, from, to)?;

        let mut from = (&self.data[from as usize]) as *const u32 as u32;
        let to = (&self.data[to as usize]) as *const u32 as u32;

        // Authorize the FPEC of Bank1 Access
        self.flash.keyr().write(|x| x.set_keyr(Self::FLASH_KEY1));
        self.flash.keyr().write(|x| x.set_keyr(Self::FLASH_KEY2));

        // Fast program mode unlock
        self.flash
            .modekeyr()
            .write(|x| x.set_modekeyr(Self::FLASH_KEY1));
        self.flash
            .modekeyr()
            .write(|x| x.set_modekeyr(Self::FLASH_KEY2));

        //erase procedure TODO (for bigger sections there are better ways)
        while from <= to {
            //set page err
            self.flash.ctlr().modify(|x| x.set_page_er(true));
            //set inital address
            self.flash.addr().write(|x| x.set_far(from));
            self.flash.ctlr().modify(|x| x.set_strt(true));
            // wait to finish
            while self.flash.statr().read().bsy() {}
            //clear error flag
            self.flash.ctlr().modify(|x| x.set_page_er(false));
            from += 256;
        }

        self.flash.ctlr().modify(|x| x.set_flock(true));
        self.flash.ctlr().modify(|x| x.set_lock(true));
        Ok(())
    }
}
