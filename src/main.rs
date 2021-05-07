use iced::{Sandbox, Settings};
use icy_redis_viewer::RedisViewer;

fn main() -> iced::Result {
    RedisViewer::run(Settings::default())
}
