use std::fs::File;
use std::ops::Add;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use memmap::{Mmap, MmapOptions};

use anyhow::Result;

fn main() -> Result<()> {
    let thread_count = thread::available_parallelism()?.get();

    let mmf = build_mmf()?;

    let mut handles = vec![];

    for thread_id in 0..1 {
        let mmf = mmf.clone();

        thread::sleep(Duration::from_millis(1_000));

        unsafe {
            let h = thread::spawn(move || process_chunk(thread_id, thread_count, mmf));
            handles.push(h);
        };

    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

unsafe fn process_chunk(thread_id: usize, thread_count: usize, mmf: Arc<Mmap>) -> Result<()> {
    let (mut cur, end) = get_start_end(thread_id, thread_count, &mmf);

    println!("ThreadId: {:?}: Start: {:?} to End {:?}", thread_id, cur as usize, end as usize);

    while cur < end {
        let mut buf = vec![];
        while *cur != 59u8 {
            buf.push(*cur);
            cur = cur.add(1);
        }
        cur = cur.add(1);

        println!("Name: {}", buf.iter().map(|u| *u as char).collect::<String>());

        let mut buf = vec![];
        while *cur != 10 {
            buf.push(*cur);
            cur = cur.add(1);
        }

        println!("Temp: {}", buf.iter().map(|u| *u as char).collect::<String>());

        cur = cur.add(1);
    }

    Ok(())
}

unsafe fn get_start_end(thread_id: usize, thread_count: usize, mmf: &Arc<Mmap>) -> (*const u8, *const u8) {
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

unsafe fn find_next_newline(cur: *const u8, mmf: &Arc<Mmap>) -> *const u8 {
    if mmf.as_ptr() == cur {
        return cur;
    }

    let mut next = cur.sub(1);
    while *next != 10u8 {
        next = next.sub(1);
    }

    next
}

fn build_mmf() -> Result<Arc<Mmap>> {
    let f = File::open("/home/bennet/git/1brc-rust/measurements.txt")?;
    Ok(Arc::new(unsafe { MmapOptions::new().map(&f)? }))
}