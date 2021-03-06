pub mod plotfile;
pub mod seekfile;

use bigint::U256;
use blake2b_simd::blake2b;
use std::cmp::min;

pub const LOOP_COUNT: usize = 8192;
pub const HASH_LEN: usize = 64;
pub const SEED_LEN: usize = 21 + 4; // addr + nonce
pub const TOTAL_LEN: usize = SEED_LEN + LOOP_COUNT * HASH_LEN;

/// return boxed slice with filled zero for poc generator
pub fn get_generator_output() -> Box<[u8]> {
    let vec = [0u8; LOOP_COUNT * HASH_LEN].to_vec();
    vec.into_boxed_slice()
}

/// return boxed slice with filled zero for poc generator
pub fn get_generator_cache() -> Box<[u8]> {
    let vec = [0u8; TOTAL_LEN].to_vec();
    vec.into_boxed_slice()
}

/// get full size poc hash
pub fn poc_generator(addr: &[u8], nonce: u32, output: &mut Box<[u8]>, cache: &mut Box<[u8]>) {
    assert_eq!(addr.len(), 21);
    assert_eq!(output.len(), LOOP_COUNT * HASH_LEN);
    assert_eq!(cache.len(), TOTAL_LEN);

    // seed ..-[addr 21bytes]-[nonce 4bytes]
    let bytes: [u8; 4] = nonce.to_le_bytes();
    cache[(TOTAL_LEN - 4)..].clone_from_slice(&bytes);
    cache[(TOTAL_LEN - SEED_LEN)..(TOTAL_LEN - 4)].clone_from_slice(addr);
    //println!("source={:?}", &source[(TOTAL_LEN-SEED_LENGTH)..]);

    // seed [hash(HASH_LENGTH)]-...-[hash0]-[addr 21bytes]-[nonce 4bytes]
    // [hashN] = blake2bp([hash(N-1)]-...-[hash0]-[addr 21bytes]-[nonce 4bytes])
    let start_index = TOTAL_LEN - SEED_LEN;
    let mut final_hash = [0u8; HASH_LEN];
    for index in 0..(LOOP_COUNT) {
        let start = start_index - index * HASH_LEN;
        let end = min(start + 1024, TOTAL_LEN);
        let hash = blake2b(&cache[start..end]);
        let hash = hash.as_bytes();
        cache[(start - HASH_LEN)..start].clone_from_slice(&hash);
    }
    {
        // generate final hash
        let hash = blake2b(&cache[..]);
        let hash = hash.as_bytes();
        final_hash.clone_from_slice(&hash);
    }
    //println!("final={:?}\nsource={:?}", final_hash, &source[..]);

    // all hash_ints XOR with final_int
    // from: [hash(HASH_LENGTH)]-...-[hash0]-[addr 21bytes]-[nonce 4bytes]
    // to  : [hash'0]- ... - [hash'(HASH_LENGTH)]
    for (index, item) in output.iter_mut().enumerate() {
        let inner_pos = index % HASH_LEN; // 0~31
        let outer_pos = index / HASH_LEN;
        let x = &final_hash[inner_pos];
        let y = &cache[(LOOP_COUNT - outer_pos - 1) * HASH_LEN + inner_pos];
        *item = x ^ y;
        //println!("{} {:?}=={:?}^{:?}", index, item, x, y);
    }
    //println!("output={:?}", &output[..]);
}

/// get scoped 32 bytes poc hash
pub fn get_poc_hash(addr: &[u8], nonce: u32, time: u32, previous_hash: &[u8]) -> Vec<u8> {
    // work = blake2b([blockTime 4bytes]-[scopeHash 32bytes]-[previousHash 32bytes])
    assert_eq!(addr.len(), 21);
    assert_eq!(previous_hash.len(), 32);
    let scope_hash = get_scope_hash(addr, nonce, previous_hash);
    let mut vec = Vec::with_capacity(4 + 32 + 32);
    let bytes: [u8; 4] = time.to_le_bytes();
    vec.extend_from_slice(&bytes);
    vec.extend_from_slice(&scope_hash);
    vec.extend_from_slice(previous_hash);
    blake2b(&vec).as_bytes()[0..32].to_vec()
}

/// get scope index 0~31
pub fn get_scope_index(previous_hash: &[u8]) -> usize {
    // index = (previous_hash to little endian 32bytes int) % scope_length
    assert_eq!(previous_hash.len(), 32);
    let mut previous_hash = previous_hash.to_owned();
    previous_hash.reverse();
    let val: U256 = previous_hash.as_slice().into();
    let div: U256 = (LOOP_COUNT * HASH_LEN / 32).into();
    let index: u32 = (val % div).into();
    index as usize
}

/// get scope_hash from full size poc hash
fn get_scope_hash(addr: &[u8], nonce: u32, previous_hash: &[u8]) -> Vec<u8> {
    debug_assert_eq!(addr.len(), 21);
    debug_assert_eq!(previous_hash.len(), 32);
    let mut output = get_generator_output();
    let mut cache = get_generator_cache();
    poc_generator(addr, nonce, &mut output, &mut cache);
    let scope = get_scope_index(previous_hash);
    let scope_hash = &output[scope * 32..scope * 32 + 32];
    scope_hash.to_vec()
}

#[cfg(test)]
mod tests {
    use crate::plotfile::*;
    use crate::seekfile::*;
    use crate::*;
    use tempfile::tempdir;

    fn s2h(s: &str) -> Vec<u8> {
        hex::decode(s).unwrap()
    }

    #[test]
    #[ignore]
    fn plotting() {
        let mut addr = [0u8; 21];
        let tmp = tempdir().unwrap();

        // generate plot file
        addr.clone_from_slice(&s2h("00df64f24d74ea98b3a6734465ea9980ae9cdb2280"));
        let start = 0;
        let end = 40;
        let unoptimized0 = plot_unoptimized_file(&addr, start, 15, tmp.path());
        let unoptimized1 = plot_unoptimized_file(&addr, 15, end, tmp.path());

        // check plot files restore
        let files = vec![unoptimized0, unoptimized1];
        let restore = PlotFile::restore_from_dir(tmp.path());
        assert_eq!(restore, files);

        // convert to optimized
        let optimized = convert_to_optimized_file(files, tmp.path());

        // calc from seek_file() by single
        let previous_hash = s2h("e34140a2ec83f237657427a98c5ab8516f75af8bc44e4c59e79e9df997df37e0");
        let target = s2h("000000000000000000000000000000000000000000000000000000ffffff0000");
        let time = 1836;
        let (nonce, work0) = seek_file(
            &optimized.path,
            start,
            end,
            &previous_hash,
            &target,
            time,
            false,
        )
        .unwrap();
        assert_eq!(nonce, 32);

        // calc from seek_file() by multi
        let (nonce_multi, work_multi) = seek_file(
            &optimized.path,
            start,
            end,
            &previous_hash,
            &target,
            time,
            true,
        )
        .unwrap();
        assert_eq!(nonce_multi, 32);
        assert_eq!(hex::encode(work_multi), hex::encode(&work0));

        // calc from get_poc_hash()
        let work1 = get_poc_hash(&addr, nonce, time, &previous_hash);
        assert_eq!(hex::encode(&work0), hex::encode(&work1));
    }

    #[test]
    fn poc() {
        // height 100000
        let work = "d8fc394861e265ff9fa43fc9de408b6a26d631b993ba73b4048bd885b0090000";
        let addr = s2h("00de6e40c12db0920348ed0ebb136e3a926bad4a3a");
        let nonce = 685;
        let time = 1579609665 - 1557883103;
        let previous_hash = s2h("df98f659f3f31cbf3494b96e44697729e3d018b6308a6de8fefa5fd4b378d025");
        let work_hash = get_poc_hash(&addr, nonce, time, &previous_hash);
        assert_eq!(hex::encode(work_hash), work);
    }
}
