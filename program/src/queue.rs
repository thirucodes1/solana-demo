use crate::state::Event;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

/// Header for an in-account event queue ring buffer.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct EventQueueHeader {
    pub head: u64,
    pub tail: u64,
    pub capacity: u64,
}

impl EventQueueHeader {
    pub fn init(&mut self, capacity: u64) {
        self.head = 0;
        self.tail = 0;
        self.capacity = capacity;
    }

    pub fn validate(&self, buf_len: usize) -> Result<(), ProgramError> {
        let capacity = usize::try_from(self.capacity).map_err(|_| ProgramError::InvalidAccountData)?;
        if capacity == 0 || capacity > buf_len / Event::serialized_size()? {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

/// Writes an event into the queue at the current tail position.
pub fn push_event(
    header: &mut EventQueueHeader,
    buf: &mut [u8],
    event: &Event,
) -> Result<(), ProgramError> {
    header.validate(buf.len())?;
    let slot_size = Event::serialized_size()?;
    let idx = (header.tail % header.capacity) as usize;
    let offset = idx * slot_size;

    let mut slice = &mut buf[offset..offset + slot_size];
    event.serialize(&mut slice)?;

    header.tail = header.tail.wrapping_add(1);
    if header.tail.wrapping_sub(header.head) > header.capacity {
        header.head = header.head.wrapping_add(1);
    }
    Ok(())
}

impl Event {
    fn serialized_size() -> Result<usize, ProgramError> {
        // Trade is the largest event variant and determines the slot size.
        let dummy = Event::Trade {
            maker: solana_program::pubkey::Pubkey::default(),
            taker: solana_program::pubkey::Pubkey::default(),
            price_lots: 0,
            base_lots: 0,
        };
        let mut data = Vec::with_capacity(128);
        dummy
            .serialize(&mut data)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        Ok(data.len())
    }
}
