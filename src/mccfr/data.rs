use crate::mccfr::bucket::Bucket;
use crate::mccfr::player::Player;
use crate::play::game::Game;

#[derive(Debug)]
pub struct Data {
    game: Game,
    bucket: Bucket,
}

impl From<(Game, Bucket)> for Data {
    fn from((game, bucket): (Game, Bucket)) -> Self {
        Self { game, bucket }
    }
}

impl Data {
    pub fn game(&self) -> &Game {
        &self.game
    }
    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }
    pub fn player(&self) -> Player {
        Player(self.game.player())
    }
}
