#[derive(Debug, Clone)]
pub struct Seat {
    pub index: usize,
    pub stack: u32,
    pub stake: u32,
    pub status: BetStatus,
    pub actor: Rc<dyn Actor>, // Weak ?
    pub hole: Hole,
}
impl Seat {
    pub fn new(actor: Rc<dyn Actor>, stack: u32, position: usize) -> Seat {
        Seat {
            index: position,
            stack,
            stake: 0,
            status: BetStatus::Playing,
            hole: Hole::new(),
            actor,
        }
    }

    pub fn cards(&self) -> &Hole {
        &self.hole
    }

    pub fn valid_actions(&self, hand: &Hand) -> Vec<Action> {
        let mut actions = Vec::with_capacity(5);
        if self.can_check(hand) {
            actions.push(Action::Check(self.index));
        }
        if self.can_fold(hand) {
            actions.push(Action::Fold(self.index));
        }
        if self.can_call(hand) {
            actions.push(Action::Call(self.index, self.to_call(hand)));
        }
        if self.can_shove(hand) {
            actions.push(Action::Shove(self.index, self.to_shove(hand)));
        }
        if self.can_raise(hand) {
            actions.push(Action::Raise(self.index, self.to_raise(hand)));
        }
        actions
    }

    pub fn to_call(&self, hand: &Hand) -> u32 {
        hand.head.table_stake() - self.stake
    }
    pub fn to_shove(&self, hand: &Hand) -> u32 {
        std::cmp::min(self.stack, hand.head.table_stack() - self.stake)
    }
    pub fn to_raise(&self, hand: &Hand) -> u32 {
        std::cmp::min(self.to_shove(hand) - 1, 5)
    }

    fn can_check(&self, hand: &Hand) -> bool {
        self.stake == hand.head.table_stake()
    }
    fn can_shove(&self, hand: &Hand) -> bool {
        self.to_shove(hand) > 0
    }
    fn can_fold(&self, hand: &Hand) -> bool {
        self.to_call(hand) > 0
    }
    fn can_raise(&self, hand: &Hand) -> bool {
        self.to_shove(hand) > self.to_call(hand) + 1
    }
    fn can_call(&self, hand: &Hand) -> bool {
        self.can_fold(hand) && self.can_raise(hand)
    }
}
impl Display for Seat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let card1 = self.hole.cards.get(0).unwrap();
        let card2 = self.hole.cards.get(1).unwrap();
        write!(
            f,
            "{:<3}{}   {}  {} {:>7}  \n",
            self.index, self.status, card1, card2, self.stack,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BetStatus {
    Playing,
    Shoved,
    Folded,
}

impl Display for BetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BetStatus::Playing => write!(f, "P"),
            BetStatus::Shoved => write!(f, "S"),
            BetStatus::Folded => write!(f, "F"),
        }
    }
}

use super::{action::Action, hand::Hand, player::Actor};
use crate::cards::hole::Hole;
use std::{
    fmt::{Debug, Display},
    rc::Rc,
};
