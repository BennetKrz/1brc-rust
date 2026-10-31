use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::sync::Arc;
use std::{env, thread};
use memmap::{Mmap, MmapOptions};

use anyhow::Result;

fn main() -> Result<()> {
    let thread_count = thread::available_parallelism()?.get();
    let mmf = build_mmf()?;

    let mut handles = vec![];
    for thread_id in 0..thread_count {
        let mmf = mmf.clone();

    handles.push(thread::spawn(move || process_chunk(thread_id, thread_count, mmf)));
    }

    let results = handles.into_iter().map(|h| h.join().unwrap().unwrap()).collect::<Vec<HashMap<String, ChunkResult>>>();
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

fn process_chunk(thread_id: usize, thread_count: usize, mmf: Arc<Mmap>) -> Result<HashMap<String, ChunkResult>> {
    unsafe {

        let (mut cur, end) = get_start_end(thread_id, thread_count, &mmf);

        //println!("ThreadId: {:?}: Start: {:?} to End {:?}", thread_id, cur as usize, end as usize);

        let mut result: HashMap<String, ChunkResult> = HashMap::new();
        while cur < end {
            // Getting the name of the city
            let mut city = vec![];
            while *cur != 59u8 {
                city.push(*cur);
                cur = cur.add(1);
            }
            cur = cur.add(1);

            let city = String::from_utf8(city)?;
            //println!("Name: {}", city);

            // Get temp
            let mut temp = vec![];
            while *cur != 10 {
                temp.push(*cur);
                cur = cur.add(1);
            }

            let temp = String::from_utf8(temp)?;
            //println!("Temp: {}", temp);
            let temp = temp.parse::<f64>()?;

            add_to_results(&mut result, &city, temp);

            cur = cur.add(1);
        }

        Ok(result)
    }
}

fn add_to_results(result: &mut HashMap<String, ChunkResult>, city: &String, temp: f64) {
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

fn get_start_end(thread_id: usize, thread_count: usize, mmf: &Arc<Mmap>) -> (*const u8, *const u8) {
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

        let mut end = find_next_newline(end, &mmf);
        if thread_id == thread_count - 1 {
            end = mmf.as_ptr().add(mmf.len());
        }
        (cur, end)
    }
}

fn find_next_newline(cur: *const u8, mmf: &Arc<Mmap>) -> *const u8 {
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
    let f = File::open(env::home_dir().unwrap().join("git/1brc-rust/measurements.txt"))?;
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