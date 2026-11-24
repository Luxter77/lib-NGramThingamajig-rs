use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

fn id_from_path(path: &Path) -> i64 {
    path.file_name().and_then(|name| name.to_str()).and_then(|name| name.split('.').next()).and_then(|stem| stem.trim().parse::<i64>().ok()).unwrap_or(1)
}

pub fn sliding_window_reader<'a>(mmap: &'a Mmap, window_size: usize) -> impl Iterator<Item = &'a [u8]> {
    mmap.windows(window_size).step_by(1)
}

pub struct Source {
    pub paths: Vec<std::path::PathBuf>,
    pub weight: u64,
    pub chain_range: std::ops::RangeInclusive<usize>,
}

pub struct SourceFile {
    pub mmap: memmap2::Mmap,
    pub weight: u64,
    pub chain_range: std::ops::RangeInclusive<usize>,
}

impl Source {
    pub fn from_glob(weight: u64, range: std::ops::RangeInclusive<usize>, glob_s: &'static str) -> Self {
        Self { paths: glob::glob(glob_s).ok().into_iter().flatten().filter_map(Result::ok).collect(), weight: weight, chain_range: range }
    }

    pub fn get_file_readers(&self) -> impl Iterator<Item = SourceFile> {
        let mut paths: Vec<_> = (&self.paths).into_iter().map(|p| (id_from_path(p), p)).collect();

        paths.sort_by_key(|(id, _)| *id);

        paths.into_iter().filter_map(|(_, path)| {
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(_) => return None,
            };

            match file.metadata() {
                Ok(meta) if meta.len() == 0 => return None,
                Err(_) => return None,
                _ => {}
            };

            let maybe_mmap = unsafe { Mmap::map(&file) };

            match maybe_mmap {
                Ok(mmap) => Some(SourceFile { mmap: mmap, weight: self.weight, chain_range: self.chain_range.clone() }),
                Err(_) => None,
            }
        })
    }
}
