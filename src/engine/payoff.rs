use super::seat::BetStatus;

#[derive(Debug, Clone)]
pub struct HandResult {
    pub seat_id: usize,
    pub status: BetStatus,
    pub staked: u32,
    pub reward: u32,
    pub score: u32,
}
