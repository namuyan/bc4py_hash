use crate::*;
use regex::Regex;
use std::cmp::min;
use std::fmt;
use std::fs::{read_dir, rename, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write, stdout};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type Address = [u8; 21];

/// working thread limit like semaphore
#[derive(Clone)]
struct Semaphore {
    count: Arc<Mutex<usize>>,
    locked: bool,
}

impl Semaphore {
    /// default limitation is cpu number
    fn new() -> Self {
        let cpu_count = num_cpus::get();
        Semaphore {
            count: Arc::new(Mutex::new(cpu_count)),
            locked: false,
        }
    }

    // acquire lock and decrement
    fn acquire(&mut self) {
        assert!(!self.locked);
        loop {
            {
                let mut count = self.count.lock().unwrap();
                if 0 < *count {
                    *count -= 1;
                    self.locked = true;
                    return;
                }
            }
            // wait for another thread release
            thread::sleep(Duration::from_secs(1));
        }
    }

    // release lock and increment
    fn release(self) {
        assert!(self.locked);
        *self.count.lock().unwrap() += 1;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlotFlag {
    Unoptimized,
    Optimized,
}

#[derive(Clone, PartialEq)]
pub struct PlotFile {
    pub flag: PlotFlag,
    pub path: PathBuf,
    pub addr: Address,
    pub start: usize,
    pub end: usize,
}

impl fmt::Debug for PlotFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PlotFile")
            .field(&self.flag)
            .field(&self.path)
            .field(&hex::encode(&self.addr))
            .field(&format!("{}-{}", self.start, self.end))
            .finish()
    }
}

impl PlotFile {
    pub fn restore_from_dir(dir: &Path) -> Vec<Self> {
        // 1=flag, 2=addr, 3=start, 4=end
        let re =
            Regex::new("^(unoptimized|optimized)\\.([a-f0-9]+)-([0-9]+)-([0-9]+)\\.dat$").unwrap();
        let mut result = vec![];
        // check the dir
        for path in read_dir(dir).unwrap() {
            let path = path.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap();
            match re.captures(name) {
                Some(c) => {
                    if c.len() != 5 {
                        continue;
                    }
                    let flag = match c.get(1).unwrap().as_str() {
                        "unoptimized" => PlotFlag::Unoptimized,
                        "optimized" => PlotFlag::Optimized,
                        _ => continue,
                    };
                    let addr = match hex::decode(c.get(2).unwrap().as_str()) {
                        Ok(addr) => {
                            let mut array: Address = [0u8; 21];
                            array.clone_from_slice(addr.as_slice());
                            array
                        }
                        Err(_) => continue,
                    };
                    let start: usize = c.get(3).unwrap().as_str().parse().unwrap();
                    let end: usize = c.get(4).unwrap().as_str().parse().unwrap();
                    result.push(PlotFile {
                        flag,
                        path,
                        addr,
                        start,
                        end,
                    })
                }
                None => continue,
            }
        }
        // ordered by start index
        result.sort_by(|a, b| {
            if a.flag == b.flag {
                a.start.cmp(&b.start)
            } else {
                match (&a.flag, &b.flag) {
                    (PlotFlag::Unoptimized, PlotFlag::Optimized) => a.start.cmp(&0usize),
                    (PlotFlag::Optimized, PlotFlag::Unoptimized) => b.start.cmp(&0usize),
                    (a, b) => panic!(format!("unexpected PlotFLag compare? {:?} with {:?}", a, b)),
                }
            }
        });
        result
    }
}

/// plot unoptimized file
///
/// recommend **SSD** for tmp_dir
pub fn plot_unoptimized_file(addr: &Address, start: usize, end: usize, tmp_dir: &Path) -> PlotFile {
    assert!(tmp_dir.is_dir());

    // create file object
    let tmp = Path::new(tmp_dir).join(format!(
        "unoptimized.{}-{}-{}.tmp",
        hex::encode(addr),
        start,
        end
    ));
    let mut fs = BufWriter::new(File::create(&tmp).unwrap());

    // generate hash
    let semaphore = Semaphore::new();
    let task_num = 1000;
    let step_size = (end - start) / task_num + 1;

    // note: block channel when disk io is busy
    let (tx, rx) = mpsc::sync_channel(4);

    // throw tasks
    let mut start_pos = start.clone();
    for _ in 0..task_num {
        let end_pos = min(end, start_pos + step_size);
        let addr = addr.clone();
        let tx = tx.clone();
        let mut semaphore = semaphore.clone();
        thread::spawn(move || {
            // wait for lock get
            semaphore.acquire();

            // generate hash
            let mut cache = get_generator_cache();
            let mut output = get_generator_output();
            let mut result = Vec::with_capacity(output.len() * step_size as usize);
            for nonce in (start_pos as u32)..(end_pos as u32) {
                poc_generator(&addr, nonce, &mut output, &mut cache);
                result.extend_from_slice(output.as_ref());
            }

            // send result
            tx.send((start_pos, end_pos, result)).unwrap();

            // release lock
            semaphore.release();
        });
        // throw next task
        start_pos = end_pos;
    }

    // wait for all thread finish
    let offset = start.clone();
    let mut step = 0usize;
    let now = Instant::now();
    for (start_pos, end_pos, result) in rx.iter().take(task_num) {
        let first_pos = LOOP_COUNT * HASH_LEN * (start_pos - offset);
        fs.seek(SeekFrom::Start(first_pos as u64)).unwrap();
        fs.write(result.as_slice()).unwrap();
        // check end position
        let calc_end_pos = fs.seek(SeekFrom::Current(0)).unwrap();
        let estimate_pos = LOOP_COUNT * HASH_LEN * (end_pos - offset);
        assert_eq!(calc_end_pos, estimate_pos as u64);

        // show progress
        step += 1;
        if cfg!(feature = "progress-bar") {
            print!("{} of {} finish {}m passed\r", step, task_num, now.elapsed().as_secs() / 60);
            stdout().flush().unwrap();
        }
    }

    // release file objext
    fs.flush().unwrap();
    std::mem::drop(fs);

    // rename XX.tmp to XX.dat
    let dst = Path::new(tmp_dir).join(format!(
        "unoptimized.{}-{}-{}.dat",
        hex::encode(addr),
        start,
        end
    ));
    rename(&tmp, &dst).unwrap();

    // return unoptimized file path
    PlotFile {
        flag: PlotFlag::Unoptimized,
        path: dst.to_path_buf(),
        addr: addr.clone(),
        start,
        end,
    }
}

/// concat some unoptimized files to one optimized file
///
/// recommend **HDD** for out_dir
pub fn convert_to_optimized_file(files: Vec<PlotFile>, out_dir: &Path) -> PlotFile {
    assert!(0 < files.len());
    assert!(out_dir.is_dir());

    // check inputs status
    let addr = files.first().unwrap().addr.clone();
    let start = files.first().unwrap().start.clone();
    let end = files.last().unwrap().end.clone();
    for (index, plot) in files.iter().enumerate().skip(1) {
        assert_eq!(plot.flag, PlotFlag::Unoptimized);
        assert_eq!(plot.addr, addr);
        let previous = files.get(index - 1).unwrap();
        assert_eq!(plot.start, previous.end);
    }

    // create file objects
    let mut reader = files
        .iter()
        .map(|plot| BufReader::new(File::open(&plot.path).unwrap()))
        .collect::<Vec<BufReader<File>>>();
    let tmp = out_dir.join(format!(
        "optimized.{}-{}-{}.tmp",
        hex::encode(addr),
        start,
        end
    ));
    let mut writer = BufWriter::new(File::create(&tmp).unwrap());

    // read and join
    let now = Instant::now();
    let mut buffer = [0u8; 32];
    let skip_size = (LOOP_COUNT * HASH_LEN - 32) as i64;
    let task_num = LOOP_COUNT * HASH_LEN / 32;
    for step in 0..task_num as u64 {
        let mut count = 0usize;
        for (fs, plot) in reader.iter_mut().zip(files.iter()) {
            // set first position to read
            fs.seek(SeekFrom::Start(step * 32)).unwrap();
            // reading..
            loop {
                match fs.read(&mut buffer) {
                    Ok(32) => {
                        count += 1;
                        writer.write(&buffer).unwrap();
                        // seek next section
                        if fs.seek(SeekFrom::Current(skip_size)).is_err() {
                            // over end of file
                            break;
                        }
                    }
                    Ok(0) => {
                        // full seeked and next file
                        break;
                    }
                    // unexpected errors
                    Ok(size) => {
                        panic!(format!("unexpected size({}bytes) reading {:?}", size, plot))
                    }
                    Err(err) => {
                        panic!(format!("error occurred on converting: {}", err.to_string()))
                    }
                }
            }
        }

        // check nonce count
        assert_eq!(count, end - start);

        // show progress
        if cfg!(feature = "progress-bar") {
            print!("{} of {} finish {}m passed\r", step, task_num, now.elapsed().as_secs() / 60);
            stdout().flush().unwrap();
        }
    }

    // release file objects
    files.into_iter().for_each(drop);
    writer.flush().unwrap();
    std::mem::drop(writer);

    // rename XX.tmp to XX.dat
    let dst = out_dir.join(format!(
        "optimized.{}-{}-{}.dat",
        hex::encode(addr),
        start,
        end
    ));
    rename(&tmp, &dst).unwrap();

    // success
    PlotFile {
        flag: PlotFlag::Optimized,
        path: dst,
        addr,
        start,
        end,
    }
}
