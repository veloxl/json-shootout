#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use rayon::prelude::*;
use rustc_hash::{FxBuildHasher, FxHashMap};
use sonic_rs::{JsonContainerTrait, Value};
use std::error::Error;
use std::hint::black_box;
use std::time::Instant;

#[global_allocator]
static GLOBAL: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[cfg(target_os = "linux")]
fn pid_res_usage_kb() -> u64 {
    probes::process_memory::current_rss().unwrap()
}

#[cfg(not(target_os = "linux"))]
fn pid_res_usage_kb() -> Result<u64, Box<dyn Error>> {
    use libproc::libproc::pid_rusage::{RUsageInfoV0, pidrusage};
    match pidrusage::<RUsageInfoV0>(std::process::id().try_into().unwrap()) {
        Ok(res) => Ok(res.ri_resident_size / 1024),
        Err(e) => Err(format!("Failed to retrieve RES memory for pid: {e}").into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let data = std::fs::read("data/dictionary.json")?;

    // Measure memory usage before deserialization
    let start_mem = pid_res_usage_kb()?;

    let start_time = Instant::now();
    let json: Value = sonic_rs::from_slice(&data)?;
    let json_object = json
        .as_object()
        .ok_or_else(|| -> Box<dyn Error> { "Root JSON value is not an object".into() })?;
    let mut json_map: FxHashMap<&str, &Value> =
        FxHashMap::with_capacity_and_hasher(json_object.len(), FxBuildHasher);
    json_object.iter().for_each(|(k, v)| {
        json_map.insert(k, v);
    });

    let duration = start_time.elapsed();

    // Calculate memory usage
    let memory = pid_res_usage_kb()? - start_mem;

    println!(
        "Loaded dictionary in {}s, size {}kB",
        duration.as_secs_f32(),
        memory
    );

    let start_time = Instant::now();
    json_map.par_iter().for_each(|(key, _)| {
        let key = *key;
        black_box(
            json_map
                .get(key)
                .copied()
                .unwrap_or_else(|| panic!("Missing key {key} during lookup")),
        );
    });
    let duration = start_time.elapsed();

    println!(
        "Looked up all keys in dictionary in {}s",
        duration.as_secs_f32()
    );

    Ok(())
}
