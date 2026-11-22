use std::fs::File;
use std::io;
use std::path::Path;

use memmap2::Mmap;

fn id_from_path(path: &Path) -> i64 {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split('.').next())
        .and_then(|stem| stem.trim().parse::<i64>().ok())
        .unwrap_or(1)
}

pub fn get_file_readers<I, P>(paths: I) -> impl Iterator<Item = io::Result<Mmap>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut md_paths: Vec<_> = paths
        .into_iter()
        .map(|p| (id_from_path(p.as_ref()), p))
        .collect();
    
    md_paths.sort_by_key(|(id, _)| *id);

    md_paths.into_iter().filter_map(|(_, path)| {
        let file = match File::open(path.as_ref()) {
            Ok(f) => f,
            Err(e) => return Some(Err(e)),
        };

        match file.metadata() {
            Ok(meta) if meta.len() == 0 => return None,
            Err(e) => return Some(Err(e)),
            _ => {}
        };

        Some(unsafe { Mmap::map(&file) })
    })
}

pub fn sliding_window_reader<'a>(
    mmap: &'a Mmap,
    window_size: usize,
) -> impl Iterator<Item = &'a [u8]> {
    mmap.windows(window_size).step_by(1)
}

pub fn decode_chopped_utf8(bytes: &[u8]) -> String {
    let mut decoder = encoding_rs::UTF_8.new_decoder();

    let mut output = String::with_capacity(bytes.len());

    let (_result, _bytes_read, _had_errors) = decoder.decode_to_string(bytes, &mut output, false);

    output
}
