//! The microscopic version of the `flop` filesystem. This is used for initrd.
//!
//! The format goes as follows:
//! - [`Header`] (currently just magic)
//! - Arbitrary number of [`HeaderEntry`] entries followed by a [`HeaderEntry`]
//!   with [`HeaderEntryType`] of [`HeaderEntryType::End`]
//! - A blob of unstructured data
#![cfg_attr(not(feature = "std"), no_std)]
use core::convert::TryInto;
use core::mem;

use fallible_iterator::FallibleIterator;
use static_assertions::const_assert;

pub const MAGIC: u64 = u64::from_le_bytes(*b"*mewing*");

type Result<T> = core::result::Result<T, Error>;
/// Errors that can happen while deserializing a microflop archive
#[derive(Debug)]
pub enum Error {
    BadMagic,
    BadEntry,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self, f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// Appears at the start of microflop files
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Header {
    /// Magic bytes, `*mewing*`
    pub magic: u64,
}

/// Offset into the file.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Offset(pub u32);

/// File name. Zero terminated UTF-8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileName(pub [u8; 15]);

typesafe_ints::int_enum_only!(
    /// Marker of whether the given header entry is the end of the header
    #[derive(Clone, Copy, Debug)]
    pub enum HeaderEntryType(u8) {
        End = 0,
        Entry = 1,
    }
);

/// Entry in the file header
#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[repr(align(8))]
pub struct HeaderEntry {
    pub fname: FileName,
    pub tag: HeaderEntryType,
    pub begin: Offset,
    pub end: Offset,
}

const_assert!(mem::size_of::<HeaderEntry>() % 8 == 0);

/// A client to access a microflop filesystem
#[derive(Debug)]
pub struct Microflop<'a> {
    /// contains the entire region of the file
    region: &'a [u8],
}

/// An iterator over the files in an archive
pub struct IterFiles<'a> {
    region: &'a [u8],
    start: &'a [u8],
}

impl<'a> FallibleIterator for IterFiles<'a> {
    type Item = (FileName, &'a [u8]);
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        let (entry, rest) = HeaderEntry::deserialize(self.start)?;
        // println!("{:?}", entry);
        Ok(match entry.tag {
            HeaderEntryType::End => None,
            HeaderEntryType::Entry => {
                self.start = rest;
                Some((
                    entry.fname,
                    &self.region[entry.begin.0 as usize..entry.end.0 as usize],
                ))
            }
        })
    }
}

/// An iterator over the entries in an archive (debugging use)
pub struct IterEntries<'a> {
    region: &'a [u8],
    start: &'a [u8],
}

impl<'a> FallibleIterator for IterEntries<'a> {
    type Item = (HeaderEntry, &'a [u8]);
    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        let (entry, rest) = HeaderEntry::deserialize(self.start)?;
        // println!("{:?}", entry);
        Ok(match entry.tag {
            HeaderEntryType::End => None,
            HeaderEntryType::Entry => {
                self.start = rest;
                Some((
                    entry,
                    &self.region[entry.begin.0 as usize..entry.end.0 as usize],
                ))
            }
        })
    }
}

impl FileName {
    pub const EMPTY: FileName = FileName([0u8; 15]);

    /// Gets the filename as a string. Fails if it is invalid.
    pub fn as_str(&self) -> Result<&str> {
        let endidx = self
            .0
            .iter()
            .position(|c| *c == b'\0')
            .ok_or(Error::BadEntry)?;

        let s = &self.0[..endidx];
        core::str::from_utf8(s).map_err(|_| Error::BadEntry)
    }

    /// Makes a new FileName. Fails if you give it a string too long.
    pub fn new(name: &str) -> Result<FileName> {
        let bytes = name.as_bytes();
        let mut out = [0u8; 15];
        if bytes.len() > 14 {
            return Err(Error::BadEntry);
        }
        out[..bytes.len()].copy_from_slice(bytes);
        Ok(FileName(out))
    }

    /// Serializes the [`FileName`] to an output stream
    #[cfg(feature = "std")]
    pub fn serialize<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&self.0)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl Offset {
    /// Serializes the [`Offset`] to an output stream
    pub fn serialize<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&self.0.to_le_bytes())?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl HeaderEntryType {
    /// Serializes the [`HeaderEntryType`] to an output stream
    pub fn serialize<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&(*self as u8).to_le_bytes())?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl HeaderEntry {
    /// Serializes the [`HeaderEntry`] to an output stream
    pub fn serialize<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        self.fname.serialize(w)?;
        self.tag.serialize(w)?;
        self.begin.serialize(w)?;
        self.end.serialize(w)?;
        Ok(())
    }
}

impl<'a> Microflop<'a> {
    pub fn new(region: &'a [u8]) -> Result<Microflop> {
        let header = region[0..mem::size_of::<Header>()]
            .try_into()
            .map_err(|_| Error::BadMagic)?;
        let magic = u64::from_le_bytes(header);
        if magic != MAGIC {
            return Err(Error::BadMagic);
        }

        Ok(Microflop { region })
    }

    pub fn files(&self) -> IterFiles<'a> {
        IterFiles {
            region: self.region,
            start: &self.region[mem::size_of::<Header>()..],
        }
    }

    pub fn entries(&self) -> IterEntries<'a> {
        IterEntries {
            region: self.region,
            start: &self.region[mem::size_of::<Header>()..],
        }
    }
}

impl HeaderEntry {
    /// Deserializes a header entry, yielding a [`HeaderEntry`] and a slice
    /// of the remaining bytes.
    fn deserialize(slice: &[u8]) -> Result<(HeaderEntry, &[u8])> {
        let (fname, rest) = slice.split_at(mem::size_of::<FileName>());
        let (tag, rest) = rest.split_at(mem::size_of::<HeaderEntryType>());
        let (begin, rest) = rest.split_at(mem::size_of::<Offset>());
        let (end, rest) = rest.split_at(mem::size_of::<Offset>());

        let fname = FileName(fname.try_into().unwrap());
        let begin = Offset(u32::from_le_bytes(begin.try_into().unwrap()));
        let end = Offset(u32::from_le_bytes(end.try_into().unwrap()));
        let tag = tag[0].try_into().map_err(|_| Error::BadEntry)?;
        Ok((
            HeaderEntry {
                fname,
                begin,
                end,
                tag,
            },
            rest,
        ))
    }
}
