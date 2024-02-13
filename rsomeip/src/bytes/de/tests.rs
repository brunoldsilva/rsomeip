#![cfg(test)]

use super::*;

#[test]
fn read() {
    let buf = [1u8, 0, 0, 0, 2];
    let mut de = Deserializer::new(&buf);
    assert_eq!(de.read().map(u8::from_be_bytes), Ok(1u8));
    assert_eq!(de.cursor, 1);
    assert_eq!(de.read().map(u32::from_be_bytes), Ok(2u32));
    assert_eq!(de.cursor, 5);
    assert_eq!(de.read().map(u8::from_be_bytes), Err(Error::BufferOverflow));
    assert_eq!(de.cursor, 5);
}

#[test]
fn limit() {
    let buf = [0u8, 1, 0, 2, 0, 3];
    let mut de = Deserializer::new(&buf);
    assert_eq!(de.push_limit(buf.len() + 1), Err(Error::BufferOverflow));
    assert_eq!(de.push_limit(std::mem::size_of::<u32>()), Ok(()));
    assert_eq!(
        de.push_limit(std::mem::size_of::<u64>()),
        Err(Error::ExceedsLimit)
    );
    assert_eq!(de.push_limit(std::mem::size_of::<u16>()), Ok(()));

    assert_eq!(de.read().map(u16::from_be_bytes), Ok(1u16));
    assert_eq!(de.read().map(u16::from_be_bytes), Err(Error::ExceedsLimit));

    assert_eq!(de.pop_limit(), Ok(()));
    assert_eq!(de.read().map(u16::from_be_bytes), Ok(2u16));
}

macro_rules! test_deserialize_basic_type {
    ($t:ty, $name:tt) => {
        #[test]
        fn $name() {
            let buf = (1 as $t).to_be_bytes();
            let mut de = Deserializer::new(&buf);
            assert_eq!(<$t>::deserialize(&mut de), Ok(1 as $t));
            assert_eq!(<$t>::deserialize(&mut de), Err(Error::BufferOverflow));
        }
    };
}

test_deserialize_basic_type!(u8, deserialize_u8);
test_deserialize_basic_type!(u16, deserialize_u16);
test_deserialize_basic_type!(u32, deserialize_u32);
test_deserialize_basic_type!(u64, deserialize_u64);
test_deserialize_basic_type!(i8, deserialize_i8);
test_deserialize_basic_type!(i16, deserialize_i16);
test_deserialize_basic_type!(i32, deserialize_i32);
test_deserialize_basic_type!(i64, deserialize_i64);
test_deserialize_basic_type!(f32, deserialize_f32);
test_deserialize_basic_type!(f64, deserialize_f64);

#[test]
fn deserialize_bool() {
    let buf = [0u8, 1];
    let mut de = Deserializer::new(&buf);
    assert_eq!(bool::deserialize(&mut de), Ok(false));
    assert_eq!(bool::deserialize(&mut de), Ok(true));
    assert_eq!(bool::deserialize(&mut de), Err(Error::BufferOverflow));
}

#[test]
fn deserialize_array() {
    let buf = [0u8, 1, 2, 3];
    let mut de = Deserializer::new(&buf);
    assert_eq!(<[u8; 4]>::deserialize(&mut de), Ok([0u8, 1, 2, 3]));
}

#[test]
fn deserialize_vec() {
    let buf = [0u8, 4, 0, 1, 2, 3];
    let mut de = Deserializer::new(&buf);
    assert_eq!(
        Vec::<u8>::deserialize_len(&mut de, LengthField::U16),
        Ok(vec![0u8, 1, 2, 3])
    );
}

#[test]
fn deserialize_utf8() {
    let buf = [0xef_u8, 0xbb, 0xbf, 0x72, 0x75, 0x73, 0x74, 0x00];
    let mut de = Deserializer::new(&buf);
    assert_eq!(String::deserialize(&mut de), Ok(String::from("rust")));
}

#[test]
fn deserialize_utf16_be() {
    let buf = [0xfe_u8, 0xff, 0x95, 0x08, 0x00, 0x00];
    let mut de = Deserializer::new(&buf);
    assert_eq!(String::deserialize(&mut de), Ok(String::from("锈")));
}

#[test]
fn deserialize_utf16_le() {
    let buf = [0xff_u8, 0xfe, 0x08, 0x95, 0x00, 0x00];
    let mut de = Deserializer::new(&buf);
    assert_eq!(String::deserialize(&mut de), Ok(String::from("锈")));
}
