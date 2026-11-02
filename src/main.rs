use memmap::{Mmap, MmapOptions};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::{env, thread};

use anyhow::Result;

const ASCII_NUMBER_OFFSET: u8 = 48;

fn main() -> Result<()> {
    let thread_count = thread::available_parallelism()?.get();
    let mmf = build_mmf()?;
    let data = &*mmf;

    let results: Vec<_> = thread::scope(|s| {
        let handles: Vec<_> = (0..thread_count)
            .map(|id| s.spawn(move || process_chunk(id, thread_count, data)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap().unwrap())
            .collect()
    });

    let final_results = analyze_results(results);

    for r in final_results {
        println!("{}={}", r.0, r.1)
    }
    Ok(())
}

fn process_chunk(
    thread_id: usize,
    thread_count: usize,
    data: &[u8],
) -> Result<HashMap<&[u8], ChunkResult>> {
    let (mut cur, end) = get_start_end(thread_id, thread_count, data);

    let mut result: HashMap<&[u8], ChunkResult> = HashMap::new();
    while cur < end {
        // Getting the name of the city
        let start = cur;
        while data[cur] != 59u8 {
            cur += 1;
        }
        let city = &data[start..cur];
        cur += 1;

        // Get temp
        let start = cur;
        while data[cur] != 10 {
            cur += 1;
        }

        let temp = &data[start..cur];
        let temp = parse_float(temp);

        add_to_results(&mut result, city, temp);

        cur += 1;
    }

    Ok(result)
}

fn parse_float(range: &[u8]) -> f64 {
    let minus = range[0] == 45;
    let skip = if minus { 2 } else { 1 };
    let mut result: f64 = (range[skip - 1] - ASCII_NUMBER_OFFSET) as f64;
    let mut count: f64 = 0.0;
    let mut before_period = true;
    for c in range.iter().skip(skip) {
        if *c == 46 {
            before_period = false;
            continue;
        }
        if !before_period {
            count += 1.0;
        }
        result *= 10.0;
        result += (*c - ASCII_NUMBER_OFFSET) as f64;
    }

    result /= count * 10.0;
    if minus {
        result *= -1.0;
    }

    result
}

fn add_to_results<'a>(result: &mut HashMap<&'a [u8], ChunkResult>, city: &'a [u8], temp: f64) {
    let chunk_result = result.entry(city).or_insert(ChunkResult {
        min: f64::MAX,
        max: f64::MIN,
        accu: 0f64,
        count: 0,
    });

    if chunk_result.min > temp {
        chunk_result.min = temp;
    }
    if chunk_result.max < temp {
        chunk_result.max = temp;
    }
    chunk_result.accu += temp;
    chunk_result.count += 1;
}

fn get_start_end(thread_id: usize, thread_count: usize, data: &[u8]) -> (usize, usize) {
    let count = data.len() / thread_count;
    let start = count * thread_id;

    let end = start + count;

    let mut cur = start;

    cur = find_prev_newline(cur, data);
    if start != 0 {
        cur += 1;
    }

    let mut end = find_prev_newline(end, data);
    if thread_id == thread_count - 1 {
        end = data.len();
    }
    (cur, end)
}

fn find_prev_newline(cur: usize, data: &[u8]) -> usize {
    if cur == 0 {
        return cur;
    }

    let mut next = cur - 1;
    while data[next] != 10u8 {
        next -= 1;
    }

    next
}

fn build_mmf() -> Result<Mmap> {
    let f = File::open(
        env::home_dir()
            .unwrap()
            .join("git/1brc-rust/measurements.txt"),
    )?;
    Ok(unsafe { MmapOptions::new().map(&f)? })
}

fn analyze_results(results: Vec<HashMap<&[u8], ChunkResult>>) -> Vec<(String, ChunkResult)> {
    let mut final_result: HashMap<&[u8], ChunkResult> = HashMap::new();
    for result in results {
        for (city, result) in result {
            let r = final_result.entry(city).or_insert(ChunkResult {
                min: f64::MAX,
                max: f64::MIN,
                accu: 0.0,
                count: 0,
            });
            r.accu += result.accu;
            r.count += result.count;
            if r.max < result.max {
                r.max = result.max;
            }
            if r.min > result.min {
                r.min = result.min;
            }
        }
    }

    let mut v: Vec<(String, ChunkResult)> = final_result
        .into_iter()
        .map(|x| (String::from_utf8_lossy(x.0).to_string(), x.1))
        .collect::<Vec<(String, ChunkResult)>>();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

struct ChunkResult {
    pub min: f64,
    pub max: f64,
    pub accu: f64,
    pub count: u64,
}

impl Display for ChunkResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1}/{:.1}/{:.1}",
            self.min,
            self.accu / self.count as f64,
            self.max
        )
    }
}
