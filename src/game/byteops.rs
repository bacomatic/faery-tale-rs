
// Byte vector operations

pub fn read_u32(data: &Vec<u8>, offset: &mut usize) -> u32 {
    let vs = &data[*offset .. *offset + 4];
    *offset += 4;
    u32::from_be_bytes(vs.try_into().unwrap())
}

pub fn read_i32(data: &Vec<u8>, offset: &mut usize) -> i32 {
    let vs = &data[*offset .. *offset + 4];
    *offset += 4;
    i32::from_be_bytes(vs.try_into().unwrap())
}

pub fn read_u16(data: &Vec<u8>, offset: &mut usize) -> u16 {
    let vs = &data[*offset .. *offset + 2];
    *offset += 2;
    u16::from_be_bytes(vs.try_into().unwrap())
}

pub fn read_i16(data: &Vec<u8>, offset: &mut usize) -> i16 {
    let vs = &data[*offset .. *offset + 2];
    *offset += 2;
    i16::from_be_bytes(vs.try_into().unwrap())
}

pub fn read_u8(data: &Vec<u8>, offset: &mut usize) -> u8 {
    let cc = data[*offset];
    *offset += 1;
    cc
}

// This only reads up to the FIRST NUL byte
// it is up to the caller to pad any remaining bytes
pub fn read_string(data: &Vec<u8>, offset: &mut usize) -> String {
    let str_start = *offset;
    let mut str_end = *offset + 1;

    while data[str_end] != 0 {
        str_end += 1;
    }

    // Adjust offset by the string size
    *offset += str_end - str_start;

    // str::from_utf8 will borrow the slice then to_string will clone
    // this avoids altering the source vector
    println!("read_string: start {str_start} -> end {str_end}");

    // for some reason passing just a NUL character to from_ut8 results in "\0"
    // instead of an empty string, IMHO this is a bug in Rust
    if str_end - str_start == 1 {
        return "".to_string();
    }

    std::str::from_utf8(&data[str_start .. str_end]).unwrap().to_string()
}
