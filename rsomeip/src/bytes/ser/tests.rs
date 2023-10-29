#![cfg(test)]

use super::*;

#[test]
fn new() {
    let mut buf = [0u8; 16];
    let ser = Serializer::new(&mut buf);
    assert_eq!(ser.buffer, [0u8; 16]);
    assert_eq!(ser.cursor, 0);
}

#[test]
fn write() {
    let mut buf = [0u8; 4];
    let mut ser = Serializer::new(&mut buf);
    let value = [1u8, 2, 3, 4];
    let res = ser.write(&value);
    assert_eq!(res, Ok(value.len()));
    assert_eq!(ser.buffer, value);
    assert_eq!(ser.cursor, value.len());
}

#[test]
fn write_multiple() {
    let mut buf = [0u8; 8];
    let mut ser = Serializer::new(&mut buf);
    let value1 = [1u8, 2, 3, 4];
    let res = ser.write(&value1);
    assert_eq!(res, Ok(value1.len()));
    assert_eq!(ser.buffer, [1u8, 2, 3, 4, 0, 0, 0, 0]);
    assert_eq!(ser.cursor, value1.len());
    let value2 = [5u8, 6, 7, 8];
    let res = ser.write(&value2);
    assert_eq!(res, Ok(value2.len()));
    assert_eq!(ser.buffer, [1u8, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(ser.cursor, value1.len() + value2.len());
}

#[test]
fn write_too_much() {
    let mut buf = [0u8; 2];
    let mut ser = Serializer::new(&mut buf);
    let value = [1u8, 2, 3, 4];
    let res = ser.write(&value);
    assert_eq!(res, Err(Error::BufferOverflow));
    assert_eq!(ser.buffer, [0u8; 2]);
    assert_eq!(ser.cursor, 0);
}

#[test]
fn skip() {
    let mut buf = [0u8; 8];
    let mut ser = Serializer::new(&mut buf);
    let res = ser.skip(4);
    assert_eq!(res, Ok(0..4));
    let value = [1u8, 2, 3, 4];
    let res = ser.write(&value);
    assert_eq!(res, Ok(4));
    assert_eq!(ser.buffer, [0u8, 0, 0, 0, 1, 2, 3, 4]);
    assert_eq!(ser.cursor, 4 + value.len());
}

#[test]
fn write_to() {
    let mut buf = [0u8; 8];
    let mut ser = Serializer::new(&mut buf);
    let skipped = ser.skip(4).expect("should skip four bytes");
    assert_eq!(skipped, 0..4);
    let value1 = [1u8, 2, 3, 4];
    let res = ser.write(&value1);
    assert_eq!(res, Ok(value1.len()));
    assert_eq!(ser.buffer, [0u8, 0, 0, 0, 1, 2, 3, 4]);
    assert_eq!(ser.cursor, 4 + value1.len());
    let value2 = [5u8, 6, 7, 8];
    let res = ser.write_to(skipped, &value2);
    assert_eq!(res, Ok(value2.len()));
    assert_eq!(ser.buffer, [5u8, 6, 7, 8, 1, 2, 3, 4]);
    assert_eq!(ser.cursor, value1.len() + value2.len());
}

macro_rules! test_serialize_basic_type {
    ($t:ty, $name:tt) => {
        #[test]
        fn $name() {
            let mut buf = [0u8; std::mem::size_of::<$t>()];
            let mut ser = Serializer::new(&mut buf);
            let res = <$t>::MAX.serialize(&mut ser);
            assert_eq!(res, Ok(()));
            assert_eq!(buf, <$t>::MAX.to_be_bytes());
        }
    };
}

test_serialize_basic_type!(u8, serialize_u8);
test_serialize_basic_type!(u16, serialize_u16);
test_serialize_basic_type!(u32, serialize_u32);
test_serialize_basic_type!(u64, serialize_u64);
test_serialize_basic_type!(i8, serialize_i8);
test_serialize_basic_type!(i16, serialize_i16);
test_serialize_basic_type!(i32, serialize_i32);
test_serialize_basic_type!(i64, serialize_i64);
test_serialize_basic_type!(f32, serialize_f32);
test_serialize_basic_type!(f64, serialize_f64);

#[test]
fn serialize_bool() {
    let mut buf = [0u8; std::mem::size_of::<bool>()];
    let mut ser = Serializer::new(&mut buf);
    let res = true.serialize(&mut ser);
    assert_eq!(res, Ok(()));
    assert_eq!(buf, [1u8]);
}

#[test]
fn serialize_vec() {
    let mut buf = [0u8; 10];
    let mut ser = Serializer::new(&mut buf);
    let value = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let res = value.serialize(&mut ser);
    assert_eq!(res, Ok(()));
    assert_eq!(&buf, &value[..]);
}

#[test]
fn serialize_array() {
    let mut buf = [0u8; 10];
    let mut ser = Serializer::new(&mut buf);
    let value = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let res = value.serialize(&mut ser);
    assert_eq!(res, Ok(()));
    assert_eq!(&buf, &value[..]);
}

#[test]
fn serialize_fn() {
    let mut buf = [0u8; 4];
    let mut ser = Serializer::new(&mut buf);
    let value = |ser: &mut Serializer| {
        1u8.serialize(ser)?;
        2u8.serialize(ser)?;
        3u8.serialize(ser)?;
        4u8.serialize(ser)
    };
    let res = value.serialize(&mut ser);
    assert_eq!(res, Ok(()));
    assert_eq!(buf, [1u8, 2, 3, 4]);
}

#[test]
fn serialize_utf8() {
    let mut buf = [0u8; 10];
    let mut ser = Serializer::new(&mut buf);
    let value = "Hello!";
    let res = value.serialize_utf8(&mut ser, None);
    assert_eq!(res, Ok(()));
    assert_eq!(
        buf,
        [
            0xef_u8, 0xbb, 0xbf, // UTF-8 Byte Order Mark
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21, // Hello!
            0x00  // Delimiter
        ]
    );
}

#[test]
fn serialize_utf16_be() {
    let mut buf = [0u8; 12];
    let mut ser = Serializer::new(&mut buf);
    let value = "语言处理";
    let res = value.serialize_utf16_be(&mut ser, None);
    assert_eq!(res, Ok(()));
    assert_eq!(
        buf,
        [
            0xfe_u8, 0xff, // UTF-16 Big Endian Byte Order Mark
            0x8b, 0xed, 0x8a, 0x00, 0x59, 0x04, 0x74, 0x06, // 语言处理
            0x00, 0x00, // Delimiter
        ]
    );
}

#[test]
fn serialize_utf16_le() {
    let mut buf = [0u8; 12];
    let mut ser = Serializer::new(&mut buf);
    let value = "语言处理";
    let res = value.serialize_utf16_le(&mut ser, None);
    assert_eq!(res, Ok(()));
    assert_eq!(
        buf,
        [
            0xff_u8, 0xfe, // UTF-16 Little Endian Byte Order Mark
            0xed, 0x8b, 0x00, 0x8a, 0x04, 0x59, 0x06, 0x74, // 语言处理
            0x00, 0x00, // Delimiter
        ]
    );
}
