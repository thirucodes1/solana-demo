use borsh::{to_vec, BorshDeserialize};
use matching_engine::{
    instruction::EngineInstruction,
    matching::match_orders,
    queue::{push_event, EventQueueHeader},
    state::{Event, Order, UserAccount},
};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[test]
fn instruction_roundtrip() {
    let ix = EngineInstruction::PlaceOrder {
        price_lots: 100,
        max_base_lots: 10,
        side_is_bid: true,
    };

    let encoded = to_vec(&ix).expect("serialize");
    let decoded = EngineInstruction::try_from_slice(&encoded).expect("deserialize");

    match decoded {
        EngineInstruction::PlaceOrder {
            price_lots,
            max_base_lots,
            side_is_bid,
        } => {
            assert_eq!(price_lots, 100);
            assert_eq!(max_base_lots, 10);
            assert!(side_is_bid);
        }
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn event_queue_rejects_invalid_capacity() {
    let event = Event::Trade {
        maker: Pubkey::new_unique(),
        taker: Pubkey::new_unique(),
        price_lots: 100,
        base_lots: 1,
    };
    let mut data = vec![0; to_vec(&event).unwrap().len()];
    let mut header = EventQueueHeader::try_from_slice(&[0; 24]).unwrap();

    assert_eq!(
        push_event(&mut header, &mut data, &event),
        Err(ProgramError::InvalidAccountData)
    );
    assert_eq!(header.tail, 0);

    header.capacity = 2;
    assert_eq!(
        push_event(&mut header, &mut data, &event),
        Err(ProgramError::InvalidAccountData)
    );

    header.capacity = 1;
    let short_len = data.len() - 1;
    assert_eq!(
        push_event(&mut header, &mut data[..short_len], &event),
        Err(ProgramError::InvalidAccountData)
    );
}

#[test]
fn event_queue_writes_trade_with_valid_capacity() {
    let event = Event::Trade {
        maker: Pubkey::new_unique(),
        taker: Pubkey::new_unique(),
        price_lots: 100,
        base_lots: 1,
    };
    let encoded = to_vec(&event).unwrap();
    let mut data = vec![0; encoded.len()];
    let mut header = EventQueueHeader {
        head: 0,
        tail: 0,
        capacity: 1,
    };

    push_event(&mut header, &mut data, &event).unwrap();
    assert_eq!(data, encoded);
    assert_eq!(header.tail, 1);
}

#[test]
fn simple_matching_flow() {
    let mut taker = UserAccount {
        owner: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        base_position: 0,
        quote_position: 0,
        last_update_ts: 0,
        open_orders: [Order::default(); 8],
    };

    let mut maker = UserAccount {
        owner: Pubkey::new_unique(),
        market: taker.market,
        base_position: 100,
        quote_position: 0,
        last_update_ts: 0,
        open_orders: [Order {
            id: 1,
            price_lots: 50,
            base_lots: 20,
            side_is_bid: false,
            is_active: true,
        }; 8],
    };

    let mut makers = vec![maker];
    let mut events = Vec::new();
    let mut max_quote_change = 0i64;

    match_orders(
        &mut taker,
        &mut makers[..],
        10,
        &mut max_quote_change,
        true,
        &mut events,
    );

    assert_eq!(taker.base_position, 10);
    assert_eq!(taker.quote_position, -500);
    assert_eq!(makers[0].base_position, 90);
    assert_eq!(makers[0].quote_position, 500);
    assert_eq!(events.len(), 1);
}
