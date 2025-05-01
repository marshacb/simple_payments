use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct ClientAccount {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl ClientAccount {
    pub fn new(client_id: u16) -> Self {
        Self {
            client: client_id,
            available: dec!(0).round_dp(4),
            held: dec!(0).round_dp(4),
            total: dec!(0).round_dp(4),
            locked: false,
        }
    }
}
