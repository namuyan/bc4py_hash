use std::cmp::min;
use std::ffi::CString;
use std::os::raw::{c_char, c_uint};

static BUF: [u8; 32] = [0u8; 32];

fn fix_output_len(data: &[u8]) -> Vec<u8> {
    let mut vec = BUF.to_vec();
    let len = min(32, data.len());
    vec[0..len].clone_from_slice(&data[0..len]);
    vec
}

#[link(name = "yespower", kind = "static")]
extern "C" {
    fn yespower_hash(input: *const c_char, output: *mut c_char, version: c_uint);
}

/// yescrypt(yespower0.5) hash
///
/// input 80 bytes vec and output 32 bytes vec
pub fn get_yescrypt_hash(input: Vec<u8>) -> Vec<u8> {
    assert_eq!(input.len(), 80);
    unsafe {
        let input_str = CString::from_vec_unchecked(input);
        // note: output buffer require 32 bytes
        let buffer = BUF.to_vec();
        let ptr = CString::from_vec_unchecked(buffer).into_raw();
        yespower_hash(input_str.as_ptr(), ptr, 1u32);
        // note: prone only first 32 bytes
        fix_output_len(CString::from_raw(ptr).as_bytes())
    }
}

/// yespower0.9 hash
///
/// input 80 bytes vec and output 32 bytes vec
pub fn get_yespower_hash(input: Vec<u8>) -> Vec<u8> {
    assert_eq!(input.len(), 80);
    unsafe {
        let input_str = CString::from_vec_unchecked(input);
        // note: output buffer require 32 bytes
        let buffer = BUF.to_vec();
        let ptr = CString::from_vec_unchecked(buffer).into_raw();
        yespower_hash(input_str.as_ptr(), ptr, 2u32);
        // note: prone only first 32 bytes
        fix_output_len(CString::from_raw(ptr).as_bytes())
    }
}

#[link(name = "x16s", kind = "static")]
extern "C" {
    fn shield_x16s_hash(input: *const c_char, output: *mut c_char);
}

/// X16S hash
///
/// input 80 bytes vec and output 32 bytes vec
pub fn get_x16s_hash(input: Vec<u8>) -> Vec<u8> {
    assert_eq!(input.len(), 80);
    unsafe {
        let input_str = CString::from_vec_unchecked(input);
        // note: output buffer require 32 bytes
        let buffer = BUF.to_vec();
        let ptr = CString::from_vec_unchecked(buffer).into_raw();
        shield_x16s_hash(input_str.as_ptr(), ptr);
        // note: prone only first 32 bytes
        fix_output_len(CString::from_raw(ptr).as_bytes())
    }
}

#[cfg(windows)]
#[link(name = "x11", kind = "static")]
extern "C" {
    fn x11_hash(input: *const c_char, output: *mut c_char);
}

/// avoid multiple definition error by GCC linker (sha3)
#[cfg(not(windows))]
#[link(name = "x11", kind = "static")]
#[allow(unused_attributes)]
#[link_args = "-z muldefs"]
extern "C" {
    fn x11_hash(input: *const c_char, output: *mut c_char);
}

/// x11 hash
///
/// input 80 bytes vec and output 32 bytes vec
pub fn get_x11_hash(input: Vec<u8>) -> Vec<u8> {
    assert_eq!(input.len(), 80);
    unsafe {
        let input_str = CString::from_vec_unchecked(input);
        // note: output buffer require 32 bytes
        let buffer = BUF.to_vec();
        let ptr = CString::from_vec_unchecked(buffer).into_raw();
        x11_hash(input_str.as_ptr(), ptr);
        // note: prone only first 32 bytes
        fix_output_len(CString::from_raw(ptr).as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn yescrypt() {
        // https://bitzeny-blockbook.ilmango.work/api/block/10000
        let input = hex::decode("0200000094510dea6171082148331425e9938ca67de1e95c5053e199b2243b0d4b0000001edca31eaba9088dabbe2c48c341b21fe3e897ef660d2f40146cffe7b96dc60ffb1d6b54b1a15b1dc0000e5a").unwrap();
        let output = "b25a34bb0ecea05e0dbe180231216a1a14ca262bf7d3211458e231863f000000".to_owned();
        let calc = get_yescrypt_hash(input);
        assert_eq!(hex::encode(calc), output);
    }

    #[test]
    fn yespower() {
        let input = hex::decode("010000005eac7f92373d6fa217ec6dc08c12c610b09a87cc7647a0b513b196348e0d9d6e4ab8afb1c1b992036d23c8acd525c77d6abce2d3fd9139ffde42677c96d34174b21e4c004d736d1e0000214e").unwrap();
        let output = "599e2ae91fbc5923bca8a023771cbad6f2fdf25d3e29597315b3cc3cf93a0000".to_owned();
        let calc = get_yespower_hash(input);
        assert_eq!(hex::encode(calc), output);
    }

    #[test]
    fn x16s() {
        let input = hex::decode("01000000994484b64def55cca3b8060e846dcb710e0acc3b64f8377d5fae9d6e3df5a05ba2f97ff17ef9f55be97b4ebdb5b71e59648137c1c883b59c1d17e49c2cd354e93f9a3e00159d051dfd7a6900").unwrap();
        let output = "2ec4220481cd664a30c063f2644ca1152fcd394a443bb0824b83433300000000".to_owned();
        let calc = get_x16s_hash(input);
        assert_eq!(hex::encode(calc), output);
    }

    #[test]
    fn x11() {
        let input = hex::decode("0100000079626c40a6caad1f1e9751a32f76930fb8d61a92f209ea4603819fb07a64ed2aa0f9c4110f8555cabf5c77e6d006161b299130a24066ca9e5eedf02ae00b7b56b24d3a00bb28061d04fff920").unwrap();
        let output = "1c5368101c34ee909c519a9de4ffc798ae2275209dee2db5f54cc2ab01000000".to_owned();
        let calc = get_x11_hash(input);
        assert_eq!(hex::encode(calc), output);
    }
}
