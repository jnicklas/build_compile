mod filetext;

use std::env::current_dir;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

pub use filetext::FileText;

#[derive(Clone, Copy)]
pub struct Span(pub usize, pub usize);

pub enum Error {
    Source(FileText, &'static str, Span),
    Io(io::Error)
}

impl From<io::Error> for Error {
    fn from(from: io::Error) -> Self {
        Error::Io(from)
    }
}

pub trait Processor {
    fn process<O: Write>(&self, input: FileText, output: &mut O) -> Result<(), Error>;
}

pub fn process_root<T: Processor>(extension: &str, processor: &T) {
    perform_processing_or_die(&current_dir().expect("cannot determin current directory"), extension, processor, false)
}

pub fn process_root_unconditionally<T: Processor>(extension: &str, processor: &T) {
    perform_processing_or_die(&current_dir().expect("cannot determin current directory"), extension, processor, true)
}

pub fn process_dir<T: Processor, P: AsRef<Path>>(path: P, extension: &str, processor: &T) {
    perform_processing_or_die(&path.as_ref(), extension, processor, false)
}

pub fn process_dir_unconditionally<T: Processor, P: AsRef<Path>>(path: P, extension: &str, processor: &T) {
    perform_processing_or_die(&path.as_ref(), extension, processor, true)
}

fn perform_processing_or_die<T: Processor>(root_dir: &Path, extension: &str, processor: &T, force_build: bool) {
    match perform_processing(root_dir, extension, processor, force_build) {
        Ok(()) => (),
        Err(error) => {
            match error {
                Error::Source(file_text, cause, span) => {
                    let (start_line, start_col) = file_text.line_col(span.0);
                    let (end_line, end_col) = file_text.line_col(span.1);

                    println!("{}:{}:{}: {}:{} error: {}",
                            file_text.path().display(),
                            start_line+1, start_col+1, end_line+1, end_col, cause);

                    let out = io::stdout();
                    let mut out = out.lock();
                    file_text.highlight(span, &mut out).unwrap();

                    exit(1);
                },
                Error::Io(ref error) => {
                    println!("{}", error)
                },
            }
        }
    }
}

fn perform_processing<T: Processor>(root_dir: &Path, extension: &str, processor: &T, force_build: bool) -> Result<(), Error> {
    let files = try!(files(root_dir, extension));
    for file in files {
        let rs_file = file.with_extension("rs");
        if force_build || try!(needs_rebuild(&file, &rs_file)) {
            try!(remove_old_file(&rs_file));

            let input_file = try!(FileText::from_path(file));
            let mut output_file = try!(fs::File::create(&rs_file));

            try!(processor.process(input_file, &mut output_file));

            try!(make_read_only(&rs_file));
        }
    }
    Ok(())
}

fn remove_old_file(rs_file: &Path) -> io::Result<()> {
    match fs::remove_file(rs_file) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Unix reports NotFound, Windows PermissionDenied!
            match e.kind() {
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied=> Ok(()),
                _ => Err(e),
            }
        }
    }
}

fn needs_rebuild(file: &Path,
                 rs_file: &Path)
                 -> io::Result<bool>
{
    return match fs::metadata(&rs_file) {
        Ok(rs_metadata) => {
            let metadata = try!(fs::metadata(&file));
            Ok(compare_modification_times(&metadata, &rs_metadata))
        }
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => Ok(true),
                _ => Err(e),
            }
        }
    };

    #[cfg(unix)]
    fn compare_modification_times(metadata: &fs::Metadata,
                                  rs_metadata: &fs::Metadata)
                                  -> bool
    {
        use std::os::unix::fs::MetadataExt;
        metadata.mtime() >= rs_metadata.mtime()
    }

    #[cfg(windows)]
    fn compare_modification_times(metadata: &fs::Metadata,
                                  rs_metadata: &fs::Metadata)
                                  -> bool
    {
        use std::os::windows::fs::MetadataExt;
        metadata.last_write_time() >= rs_metadata.last_write_time()
    }

    #[cfg(not(any(unix,windows)))]
    fn compare_modification_times(metadata: &fs::Metadata,
                                  rs_metadata: &fs::Metadata)
                                  -> bool
    {
        true
    }
}

fn make_read_only(rs_file: &Path) -> io::Result<()> {
    let rs_metadata = try!(fs::metadata(&rs_file));
    let mut rs_permissions = rs_metadata.permissions();
    rs_permissions.set_readonly(true);
    fs::set_permissions(&rs_file, rs_permissions)
}

fn files<P:AsRef<Path>>(root_dir: P, extension: &str) -> io::Result<Vec<PathBuf>> {
    let mut result = vec![];
    for entry in try!(fs::read_dir(root_dir)) {
        let entry = try!(entry);
        let file_type = try!(entry.file_type());

        let path = entry.path();

        if file_type.is_dir() {
            result.extend(try!(files(&path, extension)));
        }

        if
            file_type.is_file() &&
            path.extension().is_some() &&
            path.extension().unwrap() == extension
        {
            result.push(path);
        }
    }
    Ok(result)
}
