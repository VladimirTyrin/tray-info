use anyhow::{Result, bail};
use windows::Win32::System::Performance::{
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::{HSTRING, PCWSTR};

use super::debug_log;

const ERROR_SUCCESS: u32 = 0;

/// A PDH query with one (possibly wildcard) counter, kept open between samples
/// so each sample is a single cheap collection.
pub struct PdhCounter {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl PdhCounter {
    /// `english_path` uses English counter names, so it works on any Windows display language.
    pub fn open(english_path: &str) -> Result<Self> {
        let mut query = PDH_HQUERY::default();
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        if status != ERROR_SUCCESS {
            bail!("PdhOpenQueryW failed with status {status:#010X}");
        }
        let path = HSTRING::from(english_path);
        let mut counter = PDH_HCOUNTER::default();
        let status = unsafe { PdhAddEnglishCounterW(query, &path, 0, &mut counter) };
        if status != ERROR_SUCCESS {
            close_query(query);
            bail!("PdhAddEnglishCounterW({english_path}) failed with status {status:#010X}");
        }
        let counter = Self { query, counter };
        // Rate-based counters need a previous collection to compute the first value.
        counter.collect();
        Ok(counter)
    }

    /// Collects a fresh sample and returns `(instance name, value)` for every valid instance.
    pub fn sample_instances(&mut self) -> Option<Vec<(String, f64)>> {
        if !self.collect() {
            return None;
        }
        let mut size = 0u32;
        let mut count = 0u32;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, None)
        };
        if status != PDH_MORE_DATA {
            return None;
        }
        // The API fills a byte buffer: item structs first, then the instance name strings they
        // point to. A u64 backing store keeps the structs properly aligned.
        let mut storage = vec![0u64; (size as usize).div_ceil(size_of::<u64>())];
        let items = storage.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(items),
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let items = unsafe { std::slice::from_raw_parts(items, count as usize) };
        let samples = items
            .iter()
            .filter(|item| {
                matches!(
                    item.FmtValue.CStatus,
                    PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                )
            })
            .filter_map(|item| {
                let name = unsafe { item.szName.to_string() }.ok()?;
                let value = unsafe { item.FmtValue.Anonymous.doubleValue };
                Some((name, value))
            })
            .collect();
        Some(samples)
    }

    fn collect(&self) -> bool {
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != ERROR_SUCCESS {
            debug_log(&format!(
                "PdhCollectQueryData failed with status {status:#010X}"
            ));
        }
        status == ERROR_SUCCESS
    }
}

impl Drop for PdhCounter {
    fn drop(&mut self) {
        close_query(self.query);
    }
}

fn close_query(query: PDH_HQUERY) {
    let status = unsafe { PdhCloseQuery(query) };
    if status != ERROR_SUCCESS {
        debug_log(&format!("PdhCloseQuery failed with status {status:#010X}"));
    }
}
