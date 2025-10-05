use anyhow::Result;
use std::io::Read;

pub fn read_i32(r: &mut impl Read) -> Result<i32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

pub fn read_u8(r: &mut impl Read) -> Result<u8> {
    let mut buf = [0; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub fn read_u16(r: &mut impl Read) -> Result<u16> {
    let mut buf = [0; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

pub fn read_u32(r: &mut impl Read) -> Result<u32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub fn read_u64(r: &mut impl Read) -> Result<u64> {
    let mut buf = [0; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

pub fn read_utf8(r: &mut impl Read, size: usize) -> Result<String> {
    let mut buf = vec![0; size];
    r.read_exact(&mut buf)?;

    // fix java utf8 quirks
    let mut fixed_buf = Vec::new();
    let mut i = 0;
    loop {
        if i == size {
            break;
        }

        let b = buf[i];
        if b == 0xC0 && i < buf.len() - 1 && buf[i + 1] == 0x80 {
            fixed_buf.push(0);
            i += 1;
        } else {
            fixed_buf.push(b);
        }

        i += 1;
    }

    Ok(String::from_utf8(fixed_buf.to_vec())?)
}
