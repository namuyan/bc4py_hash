use crate::get_scope_index;
use blake2b_simd::{blake2b, Hash};
use std::cmp::min;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// seek a optimized plot file
///
/// return (nonce, workHash)
pub fn seek_file(
    path: &Path,
    start: usize,
    end: usize,
    previous_hash: &[u8],
    target: &[u8],
    time: u32,
    multi: bool,
) -> Result<(u32, Vec<u8>), String> {
    assert!(start < end);
    assert_eq!(previous_hash.len(), 32);
    assert_eq!(target.len(), 32);
    let now = Instant::now();

    // get file object
    let raw_fs = File::open(path).map_err(|err| err.to_string())?;
    let mut fs = BufReader::new(raw_fs);

    // setup first position
    let scope_index = get_scope_index(previous_hash);
    let start_pos = (scope_index * 32 * (end - start)) as u64;
    fs.seek(SeekFrom::Start(start_pos))
        .map_err(|err| err.to_string())?;

    // user select by multi thread or single thread
    if multi {
        // seek by multi thread
        let cache = prepare_cache(time, &previous_hash);
        let (tx, rx) = mpsc::channel();

        // ex. start=0, end=10, cpu_count=3 step_size=4: 0,1,2,3 ,4,5,6,7 ,8,9
        let cpu_count = num_cpus::get();
        let step_size = (end - start) / cpu_count + 1;
        let mut buffer = vec![0u8; step_size * 32];
        let mut start_pos = start.clone();

        // throw tasks to all cpus
        for _ in 0..cpu_count {
            match fs.read(&mut buffer) {
                Ok(size) => {
                    let tx = tx.clone();
                    let buffer = buffer.clone();
                    let mut cache = cache.clone();
                    let target = target.to_vec();
                    let len = size / 32;
                    let end_pos = min(start_pos + len, end);
                    thread::spawn(move || {
                        for (index, nonce) in (start_pos..end_pos).enumerate() {
                            let scope_hash = &buffer[index * 32..index * 32 + 32];
                            let raw_work = poc_hash_from_scope(scope_hash, &mut cache);
                            let work_ref = &raw_work.as_bytes()[0..32];
                            if work_check(work_ref, &target) {
                                tx.send(Some((nonce as u32, work_ref.to_vec()))).unwrap();
                                return;
                            }
                        }
                        tx.send(None).unwrap();
                    });
                    // next step
                    start_pos += len;
                }
                Err(err) => return Err(err.to_string()),
            }
        }

        // wait for all thread finish
        let mut success = None;
        for result in rx.iter().take(cpu_count) {
            match result {
                Some((nonce, work)) => {
                    if success.is_none() {
                        success.replace((nonce, work));
                    }
                    // ignore other works
                }
                None => continue,
            }
        }

        // return generated result
        success.ok_or(format!(
            "full seeked but not found enough work {}mSec",
            now.elapsed().as_millis()
        ))
    } else {
        // seek by single thread
        let mut buffer = [0u8; 32];
        let mut cache = prepare_cache(time, previous_hash);
        for nonce in start..end {
            match fs.read(&mut buffer) {
                Ok(32) => {
                    let raw_work = poc_hash_from_scope(&buffer, &mut cache);
                    let work_ref = &raw_work.as_bytes()[0..32];
                    if work_check(work_ref, target) {
                        return Ok((nonce as u32, work_ref.to_vec()));
                    }
                }
                Ok(size) => return Err(format!("wrong read size {} bytes", size)),
                Err(err) => return Err(err.to_string()),
            }
        }
        Err(format!(
            "full seeked but not found enough work {}mSec",
            now.elapsed().as_millis()
        ))
    }
}

/// prepare cache array for poc_hash_from_scope()
fn prepare_cache(time: u32, previous_hash: &[u8]) -> [u8; 4 + 32 + 32] {
    let mut cache = [0u8; 4 + 32 + 32];
    let time: [u8; 4] = time.to_le_bytes();
    cache[0..4].clone_from_slice(&time);
    // cache[4..4 + 32].clone_from_slice(scope_hash);
    cache[36..36 + 32].clone_from_slice(previous_hash);
    cache
}

/// get poc_hash from scope_hash with low-cost
#[inline]
fn poc_hash_from_scope(scope_hash: &[u8], cache: &mut [u8; 4 + 32 + 32]) -> Hash {
    // time and previous_hash is already written
    // cache[0..4].clone_from_slice(&time);
    cache[4..4 + 32].clone_from_slice(scope_hash);
    // cache[36..36 + 32].clone_from_slice(previous_hash);
    blake2b(cache.as_ref())
}

/// check the work enough lower than target (little-endian)
#[inline]
fn work_check(work: &[u8], target: &[u8]) -> bool {
    // "target > work" => true
    for (work, target) in work.iter().rev().zip(target.iter().rev()) {
        if work > target {
            return false;
        } else if work < target {
            return true;
        } else {
            continue;
        }
    }
    false
}
