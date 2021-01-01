//! Generate microflop files at the command line

use clap::Clap;
use color_eyre::eyre::{eyre, Context, Result};
use fallible_iterator::FallibleIterator;
use microflop::{FileName, Header, HeaderEntry, HeaderEntryType, Microflop, Offset};

use std::{convert::TryInto, fs, io::BufWriter, mem};
use std::{io::Write, path::PathBuf};

#[derive(Debug, Clap)]
enum SubCommand {
    /// List all the files in the file
    List {
        /// File name to open
        filename: PathBuf,
    },
    /// Hexdump all the files in the file
    Dump {
        /// File name to open
        filename: PathBuf,
    },
    /// Make a new archive
    New {
        /// Input files for archiving
        files: Vec<PathBuf>,
        #[clap(short = 'o')]
        /// Output path
        output: PathBuf,
    },
}

#[derive(Debug, Clap)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

fn list(filename: PathBuf) -> Result<()> {
    let bytes = fs::read(filename)?;
    let mf = Microflop::new(&bytes)?;
    let mut files = mf.files();
    while let Some((fname, data)) = files.next()? {
        println!("File {} - {} bytes", fname.as_str()?, data.len());
    }
    Ok(())
}

fn dump(filename: PathBuf) -> Result<()> {
    let bytes = fs::read(filename)?;
    let mf = Microflop::new(&bytes)?;
    let mut files = mf.entries();
    while let Some((entry, data)) = files.next()? {
        let fname = entry.fname;
        println!("Entry: {:?}", &entry);
        println!("File {} - {} bytes", fname.as_str()?, data.len());
        println!("{}", hexdump::HexDumper::new(data));
    }
    Ok(())
}

fn new(files: &[PathBuf], output: PathBuf) -> Result<()> {
    // construct the header bits then commit them later
    let mut file_contents = vec![];
    for file in files.iter() {
        file_contents.push(fs::read(file).wrap_err("failed to read input file")?);
    }

    let headers_end = mem::size_of::<HeaderEntry>() * (files.len() + 1) + mem::size_of::<Header>();

    let mut headers = vec![];
    let mut out_pos = headers_end;
    for (file, fc) in files.iter().zip(file_contents.iter()) {
        let remain_align = (8 - fc.len() % 8) % 8;
        let file_end = out_pos + fc.len();
        let new_end = file_end + remain_align;

        let file_name = file
            .file_name()
            .ok_or_else(|| eyre!("no file name on {:?}", file))
            .and_then(|s| {
                s.to_str()
                    .ok_or_else(|| eyre!("file name contained non unicode: {:?}", s))
            })?;
        headers.push(HeaderEntry {
            fname: FileName::new(file_name)
                .wrap_err_with(|| eyre!("bad file name {:?}", file_name))?,
            begin: Offset(out_pos.try_into().wrap_err("ran outta u32")?),
            end: Offset(file_end.try_into().wrap_err("ran outta u32")?),
            tag: HeaderEntryType::Entry,
        });
        out_pos = new_end;
    }
    headers.push(HeaderEntry {
        fname: FileName::EMPTY,
        begin: Offset(0),
        end: Offset(0),
        tag: HeaderEntryType::End,
    });

    // write it out
    let out = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(output)
        .wrap_err("Unable to open output file")?;

    let mut bw = BufWriter::new(out);
    // header
    bw.write(&microflop::MAGIC.to_le_bytes())?;
    for header in headers {
        header.serialize(&mut bw)?;
    }
    let zeros = [0u8; 7];
    for fc in file_contents {
        bw.write_all(&fc)?;
        let remain_align = (8 - fc.len() % 8) % 8;
        bw.write_all(&zeros[..remain_align])?;
    }

    Ok(())
}

pub(crate) fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Opts::parse();
    match args.subcmd {
        SubCommand::List { filename } => {
            list(filename)?;
        }
        SubCommand::Dump { filename } => {
            dump(filename)?;
        }
        SubCommand::New { files, output } => {
            new(&files, output)?;
        }
    }
    Ok(())
}
