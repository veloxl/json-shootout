use rayon::prelude::*;
use rustc_hash::FxHashMap;
use sonic_rs::{JsonContainerTrait, Value};
use std::error::Error;
use std::time::Instant;

#[global_allocator]
static GLOBAL: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[cfg(target_os = "linux")]
fn pid_res_usage_kb() -> u64 {
    probes::process_memory::current_rss().unwrap()
}

#[cfg(not(target_os = "linux"))]
fn pid_res_usage_kb() -> u64 {
    use libproc::libproc::pid_rusage::{RUsageInfoV0, pidrusage};
    match pidrusage::<RUsageInfoV0>(std::process::id() as i32) {
        Ok(res) => res.ri_resident_size / 1024,
        Err(e) => {
            eprintln!("Failed to retrieve RES memory for pid: {}", e);
            std::process::exit(1);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let data = std::fs::read("data/dictionary.json")?;

    // Measure memory usage before deserialization
    let start_mem = pid_res_usage_kb();

    let start_time = Instant::now();
    let json: Value = sonic_rs::from_slice(&data)?;

    let json_map: FxHashMap<String, sonic_rs::Value> = if let Some(obj) = json.as_object() {
        obj.iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    } else {
        return Err("Root JSON value is not an object".into());
    };

    let duration = start_time.elapsed();

    // Calculate memory usage
    let memory = pid_res_usage_kb() - start_mem;

    println!(
        "Loaded dictionary in {}s, size {}kB",
        duration.as_secs_f32(),
        memory
    );

    let keys: Vec<String> = json_map.keys().cloned().collect();

    let start_time = Instant::now();
    keys.par_iter().for_each(|key| {
        let _ = json_map.get(key);
    });
    let duration = start_time.elapsed();

    println!(
        "Looked up all keys in dictionary in {}s",
        duration.as_secs_f32()
    );

    Ok(())
}
