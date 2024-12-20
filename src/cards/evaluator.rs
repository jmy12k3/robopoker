use super::hand::Hand;
use super::kicks::Kickers;
use super::rank::Rank;
use super::ranking::Ranking;
use super::suit::Suit;

#[cfg(not(feature = "shortdeck"))]
const WHEEL: u16 = 0b_1000000001111;
#[cfg(not(feature = "shortdeck"))]
const LOWEST_STRAIGHT_RANK: Rank = Rank::Five;

#[cfg(feature = "shortdeck")]
const LOWEST_STRAIGHT_RANK: Rank = Rank::Nine;
#[cfg(feature = "shortdeck")]
const WHEEL: u16 = 0b_1000011110000;

/// A lazy evaluator for a hand's strength.
///
/// Using a compact representation of the Hand, we search for
/// the highest Value hand using bitwise operations. I should
/// benchmark this and compare to a massive HashMap<Hand, Value> lookup implementation.
pub struct Evaluator(Hand);
impl From<Hand> for Evaluator {
    fn from(h: Hand) -> Self {
        Self(h)
    }
}

impl Evaluator {
    pub fn find_ranking(&self) -> Ranking {
        None.or_else(|| self.find_flush())
            .or_else(|| self.find_4_oak())
            .or_else(|| self.find_3_oak_2_oak())
            .or_else(|| self.find_straight())
            .or_else(|| self.find_3_oak())
            .or_else(|| self.find_2_oak_2_oak())
            .or_else(|| self.find_2_oak())
            .or_else(|| self.find_1_oak())
            .expect("at least one card in Hand")
    }
    pub fn find_kickers(&self, value: Ranking) -> Kickers {
        let n = match value {
            Ranking::FourOAK(_) | Ranking::TwoPair(_, _) => 1,
            Ranking::HighCard(_) => 4,
            Ranking::OnePair(_) => 3,
            Ranking::ThreeOAK(_) => 2,
            _ => return Kickers::from(0u16),
        };
        let mask = match value {
            Ranking::TwoPair(hi, lo) => u16::from(hi) | u16::from(lo),
            Ranking::HighCard(hi)
            | Ranking::OnePair(hi)
            | Ranking::ThreeOAK(hi)
            | Ranking::FourOAK(hi) => u16::from(hi),
            _ => unreachable!(),
        };
        let mut bits = u16::from(self.0) & mask;
        while bits.count_ones() > n {
            bits &= !(1 << bits.trailing_zeros());
        }
        Kickers::from(bits)
    }

    ///

    fn find_1_oak(&self) -> Option<Ranking> {
        self.find_rank_of_n_oak(1).map(Ranking::HighCard)
    }
    fn find_2_oak(&self) -> Option<Ranking> {
        self.find_rank_of_n_oak(2).map(Ranking::OnePair)
    }
    fn find_3_oak(&self) -> Option<Ranking> {
        self.find_rank_of_n_oak(3).map(Ranking::ThreeOAK)
    }
    fn find_4_oak(&self) -> Option<Ranking> {
        self.find_rank_of_n_oak(4).map(Ranking::FourOAK)
    }
    fn find_2_oak_2_oak(&self) -> Option<Ranking> {
        self.find_rank_of_n_oak(2).and_then(|hi| {
            self.find_rank_of_n_oak_under(2, Some(hi))
                .map(|lo| Ranking::TwoPair(hi, lo))
                .or_else(|| Some(Ranking::OnePair(hi)))
        })
    }
    fn find_3_oak_2_oak(&self) -> Option<Ranking> {
        self.find_rank_of_n_oak(3).and_then(|trips| {
            self.find_rank_of_n_oak_under(2, Some(trips))
                .map(|pairs| Ranking::FullHouse(trips, pairs))
        })
    }
    fn find_straight(&self) -> Option<Ranking> {
        self.find_rank_of_straight(self.0).map(Ranking::Straight)
    }
    fn find_flush(&self) -> Option<Ranking> {
        self.find_suit_of_flush().and_then(|suit| {
            self.find_rank_of_straight_flush(suit)
                .map(Ranking::StraightFlush)
                .or_else(|| {
                    let bits = u16::from(self.0.of(&suit));
                    let rank = Rank::from(bits);
                    Some(Ranking::Flush(rank))
                })
        })
    }

    fn find_rank_of_straight(&self, hand: Hand) -> Option<Rank> {
        let wheel = WHEEL;
        let ranks = u16::from(hand);
        let mut bits = ranks;
        bits &= bits << 1;
        bits &= bits << 1;
        bits &= bits << 1;
        bits &= bits << 1;
        if bits > 0 {
            Some(Rank::from(bits))
        } else if wheel == (wheel & ranks) {
            Some(LOWEST_STRAIGHT_RANK)
        } else {
            None
        }
    }
    fn find_rank_of_straight_flush(&self, suit: Suit) -> Option<Rank> {
        let hand = self.0.of(&suit);
        self.find_rank_of_straight(hand)
    }
    fn find_suit_of_flush(&self) -> Option<Suit> {
        Suit::all()
            .map(|s| u64::from(s))
            .map(|u| u64::from(self.0) & u)
            .map(|n| n.count_ones() as u8)
            .iter()
            .position(|&n| n >= 5)
            .map(|i| Suit::from(i as u8))
    }
    fn find_rank_of_n_oak_under(&self, oak: usize, rank: Option<Rank>) -> Option<Rank> {
        let rank = rank.map(|c| u8::from(c)).unwrap_or(13) as u64;
        let mask = (1u64 << (4 * rank)) - 1;
        let hand = u64::from(self.0) & mask;
        let mut mask = 0xF << (4 * (rank)) >> 4;
        while mask > 0 {
            if oak <= (hand & mask).count_ones() as usize {
                let rank = mask.trailing_zeros() / 4;
                let rank = Rank::from(rank as u8);
                return Some(rank);
            }
            mask >>= 4;
        }
        None
    }
    fn find_rank_of_n_oak(&self, n: usize) -> Option<Rank> {
        self.find_rank_of_n_oak_under(n, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::hand::Hand;

    #[test]
    fn high_card() {
        assert!(
            Evaluator::from(Hand::try_from("As Kh Qd Jc 9s").unwrap()).find_ranking()
                == Ranking::HighCard(Rank::Ace)
        );
    }

    #[test]
    fn one_pair() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Kd Qc Js").unwrap()).find_ranking()
                == Ranking::OnePair(Rank::Ace)
        );
    }

    #[test]
    fn two_pair() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Kd Kc Qs").unwrap()).find_ranking()
                == Ranking::TwoPair(Rank::Ace, Rank::King)
        );
    }

    #[test]
    fn three_oak() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Ad Kc Qs").unwrap()).find_ranking()
                == Ranking::ThreeOAK(Rank::Ace)
        );
    }

    #[test]
    fn straight() {
        assert!(
            Evaluator::from(Hand::try_from("Ts Jh Qd Kc As").unwrap()).find_ranking()
                == Ranking::Straight(Rank::Ace)
        );
    }

    #[test]
    fn flush() {
        assert!(
            Evaluator::from(Hand::try_from("As Ks Qs Js 9s").unwrap()).find_ranking()
                == Ranking::Flush(Rank::Ace)
        );
    }

    #[test]
    fn full_house() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Ad Kc Ks").unwrap()).find_ranking()
                == Ranking::FullHouse(Rank::Ace, Rank::King)
        );
    }

    #[test]
    fn four_oak() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Ad Ac Ks").unwrap()).find_ranking()
                == Ranking::FourOAK(Rank::Ace)
        );
    }

    #[test]
    fn straight_flush() {
        assert!(
            Evaluator::from(Hand::try_from("Ts Js Qs Ks As").unwrap()).find_ranking()
                == Ranking::StraightFlush(Rank::Ace)
        );
    }

    #[test]
    #[cfg(not(feature = "shortdeck"))]
    fn wheel_straight() {
        assert!(
            Evaluator::from(Hand::try_from("As 2h 3d 4c 5s").unwrap()).find_ranking()
                == Ranking::Straight(Rank::Five)
        );
    }

    #[test]
    #[cfg(feature = "shortdeck")]
    fn shortdeck_wheel_straight() {
        assert_eq!(
            Evaluator::from(Hand::try_from("6s 7h 8d 9c As").unwrap()).find_ranking(),
            Ranking::Straight(Rank::Nine)
        );
    }

    #[test]
    #[cfg(not(feature = "shortdeck"))]
    fn wheel_straight_flush() {
        assert!(
            Evaluator::from(Hand::try_from("As 2s 3s 4s 5s").unwrap()).find_ranking()
                == Ranking::StraightFlush(Rank::Five)
        );
    }

    #[test]
    #[cfg(feature = "shortdeck")]
    fn wheel_straight_flush() {
        assert!(
            Evaluator::from(Hand::try_from("As 6s 7s 8s 9s").unwrap()).find_ranking()
                == Ranking::StraightFlush(Rank::Nine)
        );
    }

    #[test]
    fn seven_card_hand() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Kd Kc Qs Jh 9d").unwrap()).find_ranking()
                == Ranking::TwoPair(Rank::Ace, Rank::King)
        );
    }

    #[test]
    fn flush_over_straight() {
        assert!(
            Evaluator::from(Hand::try_from("4h 6h 7h 8h 9h Ts").unwrap()).find_ranking()
                == Ranking::Flush(Rank::Nine)
        );
    }

    #[test]
    fn full_house_over_flush() {
        assert!(
            Evaluator::from(Hand::try_from("Kh Ah Ad As Ks Qs Js").unwrap()).find_ranking()
                == Ranking::FullHouse(Rank::Ace, Rank::King)
        );
    }

    #[test]
    fn four_oak_over_full_house() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Ad Ac Ks Kh Qd").unwrap()).find_ranking()
                == Ranking::FourOAK(Rank::Ace)
        );
    }

    #[test]
    fn straight_flush_over_four_oak() {
        assert!(
            Evaluator::from(Hand::try_from("Ts Js Qs Ks As Ah Ad").unwrap()).find_ranking()
                == Ranking::StraightFlush(Rank::Ace)
        );
    }

    #[test]
    fn low_straight() {
        assert!(
            Evaluator::from(Hand::try_from("As 2s 3h 4d 5c 6s").unwrap()).find_ranking()
                == Ranking::Straight(Rank::Six)
        );
    }

    #[test]
    fn three_pair() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Kd Kc Qs Qh Jd").unwrap()).find_ranking()
                == Ranking::TwoPair(Rank::Ace, Rank::King)
        );
    }

    #[test]
    fn two_three_oak() {
        assert!(
            Evaluator::from(Hand::try_from("As Ah Ad Kc Ks Kh Qd").unwrap()).find_ranking()
                == Ranking::FullHouse(Rank::Ace, Rank::King)
        );
    }
}
