#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod history;
mod install;
mod platform;
mod streamlink;
mod ytdlp;

use app::{update, view, theme, State};
use iced::Size;

fn main() -> iced::Result {
    iced::application("Streamlink Video Launcher", update, view)
        .theme(theme)
        .window_size(Size::new(500.0, 400.0))
        .centered()
        .run_with(State::new)
}
