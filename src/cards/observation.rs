use super::card::Card;
use super::deck::Deck;
use super::hand::Hand;
use super::hands::HandIterator;
use super::observations::ObservationIterator;
use super::street::Street;
use super::strength::Strength;
use std::cmp::Ordering;

/// Observation represents the memoryless state of the game in between chance actions.
///
/// We store each set of cards as a Hand which does not preserve dealing order. We can
/// generate successors by considering all possible cards that can be dealt. We can calculate
/// the equity of a given hand by comparing strength all possible opponent hands.
/// This could be more memory efficient by using [Card; 2] for pocket Hands,
/// then impl From<[Card; 2]> for Hand. But the convenience of having the same Hand type is worth it.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub struct Observation {
    pocket: Hand, // if memory-bound: could be Hole/u16
    public: Hand, // if memory-bound: could be Board/[Option<Card>; 5]
}

impl Observation {
    pub fn exhaust<'a>(street: Street) -> impl Iterator<Item = Self> + 'a {
        ObservationIterator::from(street)
    }
    pub fn children<'a>(&'a self) -> impl Iterator<Item = Self> + 'a {
        let n = self.street().n_revealed();
        let removed = Hand::from(*self);
        HandIterator::from((n, removed))
            .map(|reveal| Hand::add(self.public, reveal))
            .map(|public| Self::from((self.pocket, public)))
    }
    pub fn equity(&self) -> f32 {
        assert!(self.street() == Street::Rive);
        let hand = Hand::from(*self);
        let hero = Strength::from(hand);
        let (won, sum) = HandIterator::from((2, hand))
            .map(|opponent| Hand::add(self.public, opponent))
            .map(|opponent| Strength::from(opponent))
            .map(|opponent| hero.cmp(&opponent))
            .filter(|&ord| ord != Ordering::Equal)
            .fold((0u32, 0u32), |(wins, total), ord| match ord {
                Ordering::Greater => (wins + 1, total + 1),
                Ordering::Less => (wins, total + 1),
                Ordering::Equal => unreachable!(),
            });
        match sum {
            0 => 0.5, // all draw edge case
            _ => won as f32 / sum as f32,
        }
    }
    pub fn street(&self) -> Street {
        match self.public.size() {
            0 => Street::Pref,
            3 => Street::Flop,
            4 => Street::Turn,
            5 => Street::Rive,
            _ => unreachable!("no other sizes"),
        }
    }
    pub fn pocket(&self) -> &Hand {
        &self.pocket
    }
    pub fn public(&self) -> &Hand {
        &self.public
    }
}

/// i64 isomorphism
///
/// Packs all the cards in order, starting from LSBs.
/// Good for database serialization. Interchangable with u64
impl From<Observation> for i64 {
    fn from(observation: Observation) -> Self {
        std::iter::empty::<Card>()
            .chain(observation.public.into_iter())
            .chain(observation.pocket.into_iter())
            .map(|card| 1 + u8::from(card) as u64) // distinguish 0x00 and 2c
            .fold(0u64, |acc, card| acc << 8 | card) as i64 // next card
    }
}
impl From<i64> for Observation {
    fn from(bits: i64) -> Self {
        let mut i = 0;
        let mut bits = bits as u64;
        let mut pocket = Hand::empty();
        let mut public = Hand::empty();
        while bits > 0 {
            let card = bits as u8 - 1; // distinguish 0x00 and 2c
            let card = Card::from(card);
            let hand = Hand::from(card);
            if i < 2 {
                pocket = Hand::add(pocket, hand);
            } else {
                public = Hand::add(public, hand);
            }
            bits >>= 8; // next card
            i += 1;
        }
        assert!(pocket.size() == 2);
        assert!(public.size() <= 5);
        Self::from((pocket, public))
    }
}

/// assemble Observation from private + public Hands
impl From<(Hand, Hand)> for Observation {
    fn from((pocket, public): (Hand, Hand)) -> Self {
        assert!(pocket.size() == 2);
        assert!(public.size() <= 5);
        Self { pocket, public }
    }
}

/// Generate a random observation for a given street
impl From<Street> for Observation {
    fn from(street: Street) -> Self {
        let mut deck = Deck::new();
        let n = street.n_observed();
        let public = (0..n)
            .map(|_| deck.draw())
            .map(u64::from)
            .map(Hand::from)
            .fold(Hand::empty(), Hand::add);
        let pocket = (0..2)
            .map(|_| deck.draw())
            .map(u64::from)
            .map(Hand::from)
            .fold(Hand::empty(), Hand::add);
        Self::from((pocket, public))
    }
}

/// coalesce public + private cards into single Hand
impl From<Observation> for Hand {
    fn from(observation: Observation) -> Self {
        Self::add(observation.pocket, observation.public)
    }
}

/// display Observation as pocket + public
impl std::fmt::Display for Observation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} + {}", self.pocket, self.public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bijective_i64() {
        let random = Observation::from(Street::Rive);
        assert!(random == Observation::from(i64::from(random)));
    }
}
