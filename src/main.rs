use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::sync::Arc;
use std::thread;
use memmap::{Mmap, MmapOptions};

use anyhow::Result;

fn main() -> Result<()> {
    let thread_count = thread::available_parallelism()?.get();

    let mmf = build_mmf()?;

    let mut handles = vec![];

    for thread_id in 0..thread_count {
        let mmf = mmf.clone();

        unsafe {
            let h = thread::spawn(move || process_chunk(thread_id, thread_count, mmf));
            handles.push(h);
        };
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.join().unwrap()?);
    }

    let final_results = analyze_results(results);

    for r in final_results {
        println!("{}={}", r.0, r.1)
    }
    Ok(())
}

fn analyze_results(results: Vec<HashMap<String, ChunkResult>>) -> Vec<(String, ChunkResult)> {
    let mut final_result: HashMap<String, ChunkResult> = HashMap::new();
    for result in results {
        for (city, result) in result {
            if final_result.contains_key(&city) {
                let r = final_result.get_mut(&city).unwrap();
                r.accu += result.accu;
                r.count += result.count;
                if r.max < result.max {
                    r.max = result.max;
                }
                if r.min > result.min {
                    r.min = result.min;
                }
            } else {
                final_result.insert(city, result);
            }
        }
    }

    let mut v: Vec<(String, ChunkResult)> = final_result.into_iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

unsafe fn process_chunk(thread_id: usize, thread_count: usize, mmf: Arc<Mmap>) -> Result<HashMap<String, ChunkResult>> {
    unsafe {

        let (mut cur, end) = get_start_end(thread_id, thread_count, &mmf);

        println!("ThreadId: {:?}: Start: {:?} to End {:?}", thread_id, cur as usize, end as usize);

        let mut result: HashMap<String, ChunkResult> = HashMap::new();
        while cur < end {
            // Getting the name of the city
            let mut city = vec![];
            while *cur != 59u8 {
                city.push(*cur);
                cur = cur.add(1);
            }
            cur = cur.add(1);

            let city = city.iter().map(|u| *u as char).collect::<String>();
            //println!("Name: {}", city);

            // Get temp
            let mut temp = vec![];
            while *cur != 10 {
                temp.push(*cur);
                cur = cur.add(1);
            }

            let temp = temp.iter().map(|u| *u as char).collect::<String>();
            //println!("Temp: {}", temp);
            let temp = temp.parse::<f64>()?;

            add_to_results(&mut result, &city, temp);

            cur = cur.add(1);
        }

        println!("{}", result.len());

        Ok(result)
    }
}

unsafe fn add_to_results(result: &mut HashMap<String, ChunkResult>, city: &String, temp: f64) {
    if result.contains_key(city) {
        let chunk_result = result.get_mut(city).unwrap();
        if chunk_result.min > temp {
            chunk_result.min = temp;
        }
        if chunk_result.max < temp {
            chunk_result.max = temp;
        }
        chunk_result.accu += temp;
        chunk_result.count += 1;
    } else {
        let chunk_result = ChunkResult {
            max: temp,
            min: temp,
            accu: temp,
            count: 1
        };
        result.insert(city.clone(), chunk_result);
    }
}

unsafe fn get_start_end(thread_id: usize, thread_count: usize, mmf: &Arc<Mmap>) -> (*const u8, *const u8) {
    unsafe {
        let count = mmf.len() / thread_count;
        let start = count * thread_id;

        let ptr = mmf.as_ptr();
        let end = ptr.add(start + count);

        let mut cur = ptr.add(start);

        cur = find_next_newline(cur, &mmf);
        if start != 0 {
            cur = cur.add(1);
        }
        let end = find_next_newline(end, &mmf);
        (cur, end)
    }
}

unsafe fn find_next_newline(cur: *const u8, mmf: &Arc<Mmap>) -> *const u8 {
    unsafe {
        if mmf.as_ptr() == cur {
            return cur;
        }

        let mut next = cur.sub(1);
        while *next != 10u8 {
            next = next.sub(1);
        }

        next
    }
}

fn build_mmf() -> Result<Arc<Mmap>> {
    let f = File::open("/home/bennet/git/1brc-rust/measurements.txt")?;
    Ok(Arc::new(unsafe { MmapOptions::new().map(&f)? }))
}

struct ChunkResult {
    pub min: f64,
    pub max: f64,
    pub accu: f64,
    pub count: u64
}

impl Display for ChunkResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{:.1}/{}", self.min, self.accu / self.count as f64, self.max)
    }
}