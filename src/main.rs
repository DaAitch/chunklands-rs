mod game;

use game::{Game, GameInit};

fn main() {
    env_logger::builder()
        .format_timestamp_millis()
        .format_module_path(false)
        .init();

    let mut game = Game::new(GameInit { debug: is_debug() }).unwrap();
    game.make_loop();
}

fn is_debug() -> bool {
    cfg!(debug_assertions)
}
