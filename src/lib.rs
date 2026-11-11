use std::io;

use io::Write;

use arrow_array::Array;
use arrow_array::ArrayRef;
use arrow_array::cast::AsArray;

use arrow_array::Float16Array;

use arrow_array::Int16Array;
use arrow_array::UInt16Array;

pub const NULL_BYTES: [u8; 4] = u32::to_be_bytes(0xffff_ffff);

pub trait WordLike: Array {
    fn to_word(&self, trusted_index: usize) -> [u8; 2];

    fn to_writer<W>(&self, trusted_index: usize, wtr: &mut W) -> Result<(), io::Error>
    where
        W: Write,
    {
        let is_null: bool = self.is_null(trusted_index);
        if is_null {
            wtr.write_all(&NULL_BYTES)?;
            return Ok(());
        }

        let bytes: [u8; 2] = self.to_word(trusted_index);

        let mut buf: [u8; 6] = [0; 6];
        buf[3] = 2;
        buf[4] = bytes[0];
        buf[5] = bytes[1];
        wtr.write_all(&buf)
    }
}

impl WordLike for &Float16Array {
    fn to_word(&self, trusted_index: usize) -> [u8; 2] {
        self.value(trusted_index).to_be_bytes()
    }
}

impl WordLike for &Int16Array {
    fn to_word(&self, trusted_index: usize) -> [u8; 2] {
        self.value(trusted_index).to_be_bytes()
    }
}

impl WordLike for &UInt16Array {
    fn to_word(&self, trusted_index: usize) -> [u8; 2] {
        self.value(trusted_index).to_be_bytes()
    }
}

pub fn ref2int16(r: &ArrayRef) -> Option<&Int16Array> {
    r.as_primitive_opt()
}

pub fn ref2uint16(r: &ArrayRef) -> Option<&UInt16Array> {
    r.as_primitive_opt()
}

pub fn ref2half(r: &ArrayRef) -> Option<&Float16Array> {
    r.as_primitive_opt()
}
