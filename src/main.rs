use iced::{Application, Settings};
use icy_redis_viewer::RedisViewer;

fn main() -> iced::Result {
    RedisViewer::run(Settings::default())
}
