pub struct Showdown {
    pub results: Vec<HandResult>,
    pub next_score: u32,
    pub next_stake: u32,
    pub prev_stake: u32,
}

impl Showdown {
    pub fn is_complete(&self) -> bool {
        let staked = self.results.iter().map(|p| p.staked).sum::<u32>();
        let reward = self.results.iter().map(|p| p.reward).sum::<u32>();
        staked == reward
    }
    pub fn distribute(&mut self) {
        let winnings = self.winnings();
        let mut winners = self.winners();
        let share = winnings / winners.len() as u32;
        let remainder = winnings as usize % winners.len();
        for winner in winners.iter_mut() {
            winner.reward += share;
        }
        for winner in winners.iter_mut().take(remainder as usize) {
            winner.reward += 1;
        }
    }
    pub fn next_stake(&mut self) {
        self.prev_stake = self.next_stake;
        self.next_stake = self
            .results
            .iter()
            .filter(|p| p.score == self.next_score)
            .filter(|p| p.staked > self.prev_stake)
            .filter(|p| p.status != BetStatus::Folded)
            .map(|p| p.staked)
            .min()
            .unwrap();
    }
    pub fn next_score(&mut self) {
        self.next_score = self
            .results
            .iter()
            .filter(|p| p.score < self.next_score)
            .filter(|p| p.status != BetStatus::Folded)
            .map(|p| p.score)
            .max()
            .unwrap();
    }
    fn winnings(&self) -> u32 {
        self.results
            .iter()
            .map(|p| p.staked)
            .map(|s| std::cmp::min(s, self.next_stake))
            .map(|s| s.saturating_sub(self.prev_stake))
            .sum()
    }
    fn winners(&mut self) -> Vec<&mut HandResult> {
        self.results
            .iter_mut()
            .filter(|p| p.score == self.next_score)
            .filter(|p| p.staked > self.prev_stake)
            .filter(|p| p.status != BetStatus::Folded)
            .collect()
    }
}

use super::{payoff::HandResult, seat::BetStatus};