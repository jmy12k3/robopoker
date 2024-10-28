#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Path(u64);

impl From<(usize, bool)> for Path {
    fn from((depth, raise): (usize, bool)) -> Self {
        Path((depth as u64) << 1 | raise as u64)
    }
}

impl From<u64> for Path {
    fn from(value: u64) -> Self {
        Path(value)
    }
}

impl From<Path> for u64 {
    fn from(path: Path) -> Self {
        path.0
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "H{:02}", self.0)
    }
}