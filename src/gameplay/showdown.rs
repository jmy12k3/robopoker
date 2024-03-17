// ephemeral data structure that is used to calculate the results of a hand by iterating over hand.actions to calculate side pots, handling every edge case with generalized zero-cost logic
pub struct Showdown {
    payouts: Vec<Payout>,
    next_score: u32,
    next_stake: u32,
    prev_stake: u32,
}

impl Showdown {
    pub fn new(payouts: Vec<Payout>) -> Self {
        let next_stake = u32::MIN;
        let prev_stake = u32::MIN;
        let next_score = u32::MAX;
        Showdown {
            payouts,
            next_stake,
            prev_stake,
            next_score,
        }
    }

    pub fn payouts(mut self) -> Vec<Payout> {
        loop {
            self.next_score();
            loop {
                self.next_stake();
                self.distribute();
                if self.is_complete() {
                    return self.payouts;
                }
            }
        }
    }

    fn is_complete(&self) -> bool {
        let staked = self.payouts.iter().map(|p| p.staked).sum::<u32>();
        let reward = self.payouts.iter().map(|p| p.reward).sum::<u32>();
        staked == reward
    }
    fn winnings(&self) -> u32 {
        self.payouts
            .iter()
            .map(|p| p.staked)
            .map(|s| std::cmp::min(s, self.next_stake))
            .map(|s| s.saturating_sub(self.prev_stake))
            .sum()
    }
    fn distribute(&mut self) {
        let winnings = self.winnings();
        let mut winners = self.winners();
        let share = winnings / winners.len() as u32;
        for winner in winners.iter_mut() {
            winner.reward += share;
        }
        let remainder = winnings as usize % winners.len();
        for winner in winners.iter_mut().take(remainder) {
            winner.reward += 1;
        }
    }
    fn next_stake(&mut self) {
        self.prev_stake = self.next_stake;
        self.next_stake = self
            .payouts
            .iter()
            .filter(|p| p.score == self.next_score)
            .filter(|p| p.staked > self.prev_stake)
            .filter(|p| p.status != BetStatus::Folded)
            .map(|p| p.staked)
            .min()
            .unwrap();
    }
    fn next_score(&mut self) {
        self.next_score = self
            .payouts
            .iter()
            .filter(|p| p.score < self.next_score)
            .filter(|p| p.status != BetStatus::Folded)
            .map(|p| p.score)
            .max()
            .unwrap();
    }
    fn winners(&mut self) -> Vec<&mut Payout> {
        self.payouts
            .iter_mut()
            .filter(|p| p.score == self.next_score)
            .filter(|p| p.staked > self.prev_stake)
            .filter(|p| p.status != BetStatus::Folded)
            .collect()
    }
}

use super::{payout::Payout, seat::BetStatus};