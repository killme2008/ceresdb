// Copyright 2022 CeresDB Project Authors. Licensed under Apache-2.0.

//! Log entries definition.

use std::fmt::Debug;

use common_types::{
    bytes::{MemBuf, MemBufMut},
    SequenceNumber,
};

use crate::manager::RegionId;

pub trait Payload: Send + Sync + Debug {
    type Error: std::error::Error + Send + Sync + 'static;
    /// Compute size of the encoded payload.
    fn encode_size(&self) -> usize;
    /// Append the encoded payload to the `buf`.
    fn encode_to<B: MemBufMut>(&self, buf: &mut B) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct LogEntry<P> {
    pub sequence: SequenceNumber,
    pub payload: P,
}

/// An entry to be written into the Wal.
///
/// Generally, the `payload` is a lazily encoder whose constraint is
/// `PayloadEncoder`. `region_id` is a logically region and set it as 0 if
/// unnecessary.
#[derive(Debug)]
pub struct LogWriteEntry<P> {
    pub payload: P,
}

/// A batch of `LogWriteEntry`s.
#[derive(Debug)]
pub struct LogWriteBatch<P> {
    pub(crate) region_id: RegionId,
    pub(crate) entries: Vec<LogWriteEntry<P>>,
}

impl<P: Payload> LogWriteBatch<P> {
    pub fn new(region_id: RegionId) -> Self {
        Self::with_capacity(region_id, 0)
    }

    pub fn with_capacity(region_id: RegionId, cap: usize) -> Self {
        Self {
            region_id,
            entries: Vec::with_capacity(cap),
        }
    }

    #[inline]
    pub fn push(&mut self, entry: LogWriteEntry<P>) {
        self.entries.push(entry)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear()
    }
}

impl<P: Payload> Default for LogWriteBatch<P> {
    fn default() -> Self {
        Self::new(0)
    }
}

pub trait PayloadDecoder: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type Target: Send + Sync;
    /// Decode `Target` from the `bytes`.
    fn decode<B: MemBuf>(&self, buf: &mut B) -> Result<Self::Target, Self::Error>;
}
