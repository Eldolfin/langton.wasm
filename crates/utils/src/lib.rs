#[derive(Debug, Clone, Copy, Default)]
pub enum Direction {
    #[default]
    North,
    Est,
    South,
    West,
}

impl Direction {
    pub fn left(self) -> Self {
        match self {
            Direction::North => Direction::West,
            Direction::Est => Self::North,
            Direction::South => Self::Est,
            Direction::West => Self::South,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Direction::North => Direction::Est,
            Direction::Est => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }
}
