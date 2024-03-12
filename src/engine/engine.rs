pub struct Engine {
    n_hands: u32,
}

impl Engine {
    pub fn new() -> Self {
        Engine { n_hands: 0 }
    }

    pub fn add(&mut self, seat: Seat) {
        todo!()
    }

    pub fn play(&mut self, hand: &mut Hand) {
        loop {
            if self.has_exhausted_hands(hand) {
                break;
            }
            self.start_hand(hand);
            loop {
                if self.has_exhausted_streets(hand) {
                    break;
                }
                self.start_street(hand);
                loop {
                    if self.has_exhausted_turns(hand) {
                        break;
                    }
                    self.end_turn(hand);
                }
                self.end_street(hand);
            }
            self.end_hand(hand);
        }
    }

    fn start_street(&self, hand: &mut Hand) {
        hand.head.start_street();
    }
    fn start_hand(&self, hand: &mut Hand) {
        println!("HAND  {}\n", self.n_hands);
        hand.beg_hand();
        hand.head.start_hand();
        hand.post_blinds();
        hand.deal_holes();
    }

    fn end_turn(&self, hand: &mut Hand) {
        let seat = hand.head.next();
        let action = seat.actor.act(seat, hand);
        hand.apply(action);
    }
    fn end_street(&self, hand: &mut Hand) {
        hand.head.end_street();
        hand.deal_board();
    }
    fn end_hand(&mut self, hand: &mut Hand) {
        self.n_hands += 1;
        hand.settle();
        println!("{}", hand.head);
    }

    fn has_exhausted_turns(&self, hand: &Hand) -> bool {
        !hand.head.has_more_players()
    }
    fn has_exhausted_streets(&self, hand: &Hand) -> bool {
        !hand.head.has_more_streets()
    }
    fn has_exhausted_hands(&self, hand: &Hand) -> bool {
        !hand.head.has_more_hands() || self.n_hands > 10000
    }
}

use super::{hand::Hand, seat::Seat};
