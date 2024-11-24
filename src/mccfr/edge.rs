use crate::gameplay::action::Action;
use crate::mccfr::odds::Odds;
use crate::{Arbitrary, Chips};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum Edge {
    Draw,
    Fold,
    Check,
    Call,
    Raise(Odds),
    Shove,
}

impl Edge {
    pub fn is_shove(&self) -> bool {
        matches!(self, Edge::Shove)
    }
    pub fn is_raise(&self) -> bool {
        matches!(self, Edge::Raise(_))
    }
    pub fn is_chance(&self) -> bool {
        matches!(self, Edge::Draw)
    }
    pub fn is_aggro(&self) -> bool {
        self.is_raise() || self.is_shove()
    }
    pub fn is_choice(&self) -> bool {
        !self.is_chance()
    }
}

impl From<Action> for Edge {
    fn from(action: Action) -> Self {
        match action {
            Action::Fold => Edge::Fold,
            Action::Check => Edge::Check,
            Action::Call(_) => Edge::Call,
            Action::Draw(_) => Edge::Draw,
            Action::Shove(_) => Edge::Shove,
            Action::Raise(_) => panic!("raise must be converted from odds"),
            Action::Blind(_) => panic!("blinds are not in any MCCFR trees"),
        }
    }
}
impl From<Odds> for Edge {
    fn from(odds: Odds) -> Self {
        Edge::Raise(odds)
    }
}

/// usize bijection
impl From<Edge> for u8 {
    fn from(edge: Edge) -> Self {
        match edge {
            Edge::Draw => 1,
            Edge::Fold => 2,
            Edge::Check => 3,
            Edge::Call => 4,
            Edge::Shove => 5,
            Edge::Raise(odds) => {
                6 + Odds::GRID
                    .iter()
                    .position(|&o| o == odds)
                    .expect("invalid odds value") as u8
            }
        }
    }
}
impl From<u8> for Edge {
    fn from(value: u8) -> Self {
        match value {
            1 => Edge::Draw,
            2 => Edge::Fold,
            3 => Edge::Check,
            4 => Edge::Call,
            5 => Edge::Shove,
            i @ 6..=15 => Edge::Raise(Odds::GRID[i as usize - 6]),
            _ => unreachable!("invalid edge encoding"),
        }
    }
}

/// u64 bijection
impl From<u64> for Edge {
    fn from(value: u64) -> Self {
        // Use first 3 bits for variant tag
        match value & 0b111 {
            0 => Self::Draw,
            1 => Self::Fold,
            2 => Self::Check,
            3 => Self::Call,
            4 => Self::Raise(Odds(
                ((value >> 3) & 0xFF) as Chips,
                ((value >> 11) & 0xFF) as Chips,
            )),
            5 => Self::Shove,
            _ => unreachable!(),
        }
    }
}
impl From<Edge> for u64 {
    fn from(edge: Edge) -> Self {
        match edge {
            Edge::Draw => 0,
            Edge::Fold => 1,
            Edge::Check => 2,
            Edge::Call => 3,
            Edge::Raise(Odds(num, den)) => 4 | ((num as u64) << 3) | ((den as u64) << 11),
            Edge::Shove => 5,
        }
    }
}

impl std::fmt::Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use colored::*;
        match self {
            Edge::Draw => write!(f, "{}", "─────────".dimmed()),
            Edge::Fold => write!(f, "{}", "FOLD     ".red()),
            Edge::Call => write!(f, "{}", "CALL     ".blue()),
            Edge::Check => write!(f, "{}", "CHECK    ".green()),
            Edge::Shove => write!(f, "{}", "SHOVE    ".magenta()),
            Edge::Raise(Odds(a, b)) => write!(f, "{}", format!("RAISE {}:{}", a, b).yellow()),
        }
    }
}

#[cfg(test)]
mod bijection_tests {
    use super::*;

    #[test]
    fn bijective_usize() {
        let raise = Odds::GRID.map(Edge::Raise);
        let edges = [Edge::Draw, Edge::Fold, Edge::Check, Edge::Call, Edge::Shove];
        assert!(edges
            .into_iter()
            .chain(raise)
            .all(|edge| edge == Edge::from(u8::from(edge))));
    }

    #[test]
    fn bijective_u64() {
        let raise = Odds::GRID.map(Edge::Raise);
        let edges = [Edge::Draw, Edge::Fold, Edge::Check, Edge::Call, Edge::Shove];
        assert!(edges
            .into_iter()
            .chain(raise)
            .all(|edge| edge == Edge::from(u64::from(edge))));
    }
}

impl Arbitrary for Edge {
    fn random() -> Self {
        use rand::Rng;
        match rand::thread_rng().gen_range(0..6) {
            0 => Self::Draw,
            1 => Self::Fold,
            2 => Self::Check,
            3 => Self::Call,
            4 => Self::Raise(crate::mccfr::odds::Odds::from((1, 1))),
            _ => Self::Shove,
        }
    }
}
