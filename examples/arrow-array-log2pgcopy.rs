use std::io;
use std::process::ExitCode;

use io::Write;

use arrow_array::RecordBatch;
use arrow_array::record_batch;

use arrow_array::ArrayRef;

use arrow_array::Float16Array;
use arrow_array::Int16Array;
use arrow_array::UInt16Array;

use rs_arrow_words2pgcopy::ref2half;
use rs_arrow_words2pgcopy::ref2int16;
use rs_arrow_words2pgcopy::ref2uint16;

use rs_arrow_words2pgcopy::WordLike;

fn io_record_batch() -> impl FnMut() -> Result<RecordBatch, io::Error> {
    || {
        record_batch!(
            ("id", Int16, [2, 3, 5, 7]),
            ("timestamp", Int16, [0, 0, 1, 2]),
            ("severity", UInt16, [9, 13, 17, 21]),
            ("is_active", Int16, [1, 0, 1, 0]), // Changed from Boolean to Int16
            ("tag", Int16, [Some(42), None, Some(43), Some(44)]),
            ("category_id", UInt16, [100, 200, 300, 400])
        )
        .map_err(io::Error::other)
    }
}

#[derive(Debug, PartialEq)]
enum WordArray<'a> {
    Ignore,
    Int16s(&'a Int16Array),
    UInt16s(&'a UInt16Array),
    Halfs(&'a Float16Array),
}

impl<'a> WordArray<'a> {
    fn to_writer<W>(&self, trusted_index: usize, wtr: &mut W) -> Result<(), io::Error>
    where
        W: Write,
    {
        match self {
            Self::Int16s(l) => l.to_writer(trusted_index, wtr),
            Self::UInt16s(l) => l.to_writer(trusted_index, wtr),
            Self::Halfs(l) => l.to_writer(trusted_index, wtr),
            _ => Ok(()),
        }
    }
}

fn io_bat2stdout(rbat: RecordBatch) -> impl FnOnce() -> Result<(), io::Error> {
    move || {
        let rowcnt: usize = rbat.num_rows();
        let cols: &[ArrayRef] = rbat.columns();

        let mapd: Vec<WordArray> = cols
            .iter()
            .map(|aref| {
                if let Some(i) = ref2int16(aref) {
                    return WordArray::Int16s(i);
                }

                if let Some(u) = ref2uint16(aref) {
                    return WordArray::UInt16s(u);
                }

                if let Some(f) = ref2half(aref) {
                    return WordArray::Halfs(f);
                }

                WordArray::Ignore
            })
            .collect();

        let o = io::stdout();
        let mut ol = o.lock();

        let mut colsize: [u8; 2] = [0; 2];
        colsize[1] = mapd.len() as u8;

        for i in 0..rowcnt {
            ol.write_all(&colsize)?;
            for col in mapd.iter() {
                col.to_writer(i, &mut ol)?;
            }
        }
        Ok(())
    }
}

const PGCOPY_SIGN: &[u8] = b"PGCOPY\n\xff\r\n\0";
const PGCOPY_FLAGS: [u8; 4] = [0; 4];
const PGCOPY_EXT: [u8; 4] = [0; 4];

const PGCOPY_TRAILER: [u8; 2] = u16::to_be_bytes(0xffff);

fn io_header2stdout() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        let o = io::stdout();
        let mut ol = o.lock();
        let mut buf: [u8; 19] = [0; 19];
        let src = PGCOPY_SIGN.iter().chain(&PGCOPY_FLAGS).chain(&PGCOPY_EXT);
        let z = buf.iter_mut().zip(src);
        for pair in z {
            let (dst, src) = pair;
            *dst = *src;
        }
        ol.write_all(&buf)?;
        ol.flush()
    }
}

fn io_trailer2stdout() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        let o = io::stdout();
        let mut ol = o.lock();
        ol.write_all(&PGCOPY_TRAILER)?;
        ol.flush()
    }
}

fn io_main() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        io_header2stdout()()?;
        let rbat: RecordBatch = io_record_batch()()?;
        io_bat2stdout(rbat)()?;
        io_trailer2stdout()()?;
        Ok(())
    }
}

fn sub() -> Result<(), io::Error> {
    io_main()()
}

fn main() -> ExitCode {
    sub().map(|_| ExitCode::SUCCESS).unwrap_or_else(|e| {
        eprintln!("{e}");
        ExitCode::FAILURE
    })
}
