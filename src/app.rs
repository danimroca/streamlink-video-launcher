use crate::history::{self, HistoryEntry};
use crate::install;
use crate::platform;
use crate::streamlink;
use crate::ytdlp;
use iced::{
    widget::{
        self, container, pick_list, scrollable, text, text_input, Column, Row,
    },
    Background, Border, Color, Element, Length, Task, Theme,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Best,
    P1080,
    P720,
    P480,
    P360,
    AudioOnly,
    Worst,
}

impl Quality {
    const ALL: &'static [Quality] = &[
        Quality::Best,
        Quality::P1080,
        Quality::P720,
        Quality::P480,
        Quality::P360,
        Quality::AudioOnly,
        Quality::Worst,
    ];

    fn label(&self) -> &str {
        match self {
            Quality::Best => "best",
            Quality::P1080 => "1080p",
            Quality::P720 => "720p",
            Quality::P480 => "480p",
            Quality::P360 => "360p",
            Quality::AudioOnly => "audio-only",
            Quality::Worst => "worst",
        }
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SourceChanged(String),
    QualityChanged(Quality),
    WatchPressed,
    DownloadPressed,
    StreamResolved(Result<String, String>),
    DownloadPathChosen(Option<String>),
    ToggleHistory,
    CloseHistory,
    ClearHistoryPressed,
    ConfirmClearHistory,
    CancelClearHistory,
    HistoryEntryClicked(String),
    InstallAccept,
    InstallDismiss,
    InstallFinished(Result<install::Report, String>),
}

#[derive(Debug, Clone)]
enum Status {
    Idle,
    Resolving,
    Ready,
    Error(String),
}

#[derive(Debug, Clone)]
enum InstallStage {
    Prompt,
    Progress,
    Done(install::Report),
}

pub struct State {
    source: String,
    quality: Quality,
    history: Vec<HistoryEntry>,
    history_expanded: bool,
    show_clear_confirmation: bool,
    status: Status,
    player: Option<String>,
    streamlink_available: bool,
    ytdlp_available: bool,
    history_path: PathBuf,
    install: Option<InstallStage>,
}

impl State {
    pub fn new() -> (Self, Task<Message>) {
        let data_dir = dirs::data_dir()
            .map(|p| p.join("streamlink-video-launcher"))
            .unwrap_or_else(|| PathBuf::from("."));
        let history_path = data_dir.join("history.json");
        let player = platform::find_player();
        let streamlink_available = streamlink::exists();
        let ytdlp_available = ytdlp::exists();
        let history = history::load(&history_path);

        let install = if !streamlink_available || !ytdlp_available {
            Some(InstallStage::Prompt)
        } else {
            None
        };

        (
            Self {
                source: String::new(),
                quality: Quality::Best,
                history,
                history_expanded: false,
                show_clear_confirmation: false,
                status: Status::Idle,
                player,
                streamlink_available,
                ytdlp_available,
                history_path,
                install,
            },
            Task::none(),
        )
    }

    fn source_is_url(&self) -> bool {
        self.source.contains("://")
    }

    fn can_watch(&self) -> bool {
        !self.source.is_empty()
            && (self.source_is_url() || std::path::Path::new(&self.source).exists())
    }

    fn can_download(&self) -> bool {
        !self.source.is_empty() && self.source_is_url()
    }
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SourceChanged(source) => {
            state.source = source;
            state.status = Status::Idle;
            Task::none()
        }
        Message::QualityChanged(quality) => {
            state.quality = quality;
            Task::none()
        }
        Message::WatchPressed => {
            if state.source.is_empty() {
                return Task::none();
            }

            if state.source_is_url() {
                if state.player.is_none() {
                    state.status = Status::Error(
                        "No video player found (tried mpv, vlc, celluloid)".to_string(),
                    );
                    return Task::none();
                }

                let is_youtube = platform::is_youtube_url(&state.source);
                let backend_ok = if is_youtube { state.ytdlp_available } else { state.streamlink_available };
                if !backend_ok {
                    let name = if is_youtube { "yt-dlp" } else { "streamlink" };
                    state.status = Status::Error(
                        format!("{name} is not installed or not found on PATH"),
                    );
                    return Task::none();
                }

                state.status = Status::Resolving;
                let url = normalize_url(&state.source);
                let quality = state.quality.label().to_string();
                let player = state.player.clone().unwrap();
                Task::perform(
                    async move { resolve_and_watch(&url, &quality, &player) },
                    Message::StreamResolved,
                )
            } else {
                let path = state.source.clone();
                Task::perform(
                    async move {
                        platform::open_in_default_player(&path)
                            .map(|_| String::new())
                            .map_err(|e| e)
                    },
                    Message::StreamResolved,
                )
            }
        }
        Message::StreamResolved(result) => {
            match result {
                Ok(title) => {
                    if state.source_is_url() && !title.is_empty() {
                        let entry = HistoryEntry {
                            url: state.source.clone(),
                            title,
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        state.history.push(entry);
                        history::save(&state.history_path, &state.history);
                    }
                    state.status = Status::Ready;
                }
                Err(e) => {
                    state.status = Status::Error(e);
                }
            }
            Task::none()
        }
        Message::DownloadPressed => {
            if !state.can_download() {
                return Task::none();
            }
            let need_streamlink = !platform::is_youtube_url(&state.source);
            if need_streamlink && !state.streamlink_available
                || !need_streamlink && !state.ytdlp_available
            {
                return Task::none();
            }
            state.status = Status::Resolving;
            Task::perform(
                async move {
                    rfd::FileDialog::new()
                        .set_title("Save video as...")
                        .save_file()
                        .map(|p| p.to_string_lossy().to_string())
                },
                Message::DownloadPathChosen,
            )
        }
        Message::DownloadPathChosen(path) => {
            if let Some(path) = path {
                let is_youtube = platform::is_youtube_url(&state.source);
                let url = normalize_url(&state.source);
                let quality = state.quality.label().to_string();
                if is_youtube {
                    let format = ytdlp::format_for_quality(&quality);
                    std::thread::spawn(move || {
                        let _ = ytdlp::download(&url, &format, &path);
                    });
                } else {
                    std::thread::spawn(move || {
                        let _ = streamlink::download(&url, &quality, &path);
                    });
                }
                state.status = Status::Ready;
            } else {
                state.status = Status::Idle;
            }
            Task::none()
        }
        Message::ToggleHistory => {
            state.history_expanded = !state.history_expanded;
            state.show_clear_confirmation = false;
            Task::none()
        }
        Message::CloseHistory => {
            state.history_expanded = false;
            state.show_clear_confirmation = false;
            Task::none()
        }
        Message::ClearHistoryPressed => {
            state.show_clear_confirmation = true;
            Task::none()
        }
        Message::ConfirmClearHistory => {
            state.history.clear();
            history::save(&state.history_path, &state.history);
            state.history_expanded = false;
            state.show_clear_confirmation = false;
            Task::none()
        }
        Message::CancelClearHistory => {
            state.show_clear_confirmation = false;
            Task::none()
        }
        Message::HistoryEntryClicked(url) => {
            state.source = url;
            state.history_expanded = false;
            Task::none()
        }
        Message::InstallAccept => {
            state.install = Some(InstallStage::Progress);
            Task::perform(do_install(), Message::InstallFinished)
        }
        Message::InstallDismiss => {
            let report = match state.install.take() {
                Some(InstallStage::Done(r)) => Some(r),
                _ => None,
            };
            state.install = None;
            if let Some(report) = report {
                let dir = report.bin_dir;
                if !dir.as_os_str().is_empty() {
                    if let Some(s) = dir.to_str() {
                        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
                            let path_var = std::env::var("PATH").unwrap_or_default();
                            if !path_var.contains(s) {
                                state.status = Status::Error(
                                    format!("Add {} to your PATH or restart your shell", s),
                                );
                            }
                        }
                    }
                }
            }
            Task::none()
        }
        Message::InstallFinished(result) => {
            match result {
                Ok(report) => {
                    state.streamlink_available = streamlink::exists();
                    state.ytdlp_available = ytdlp::exists();
                    state.install = Some(InstallStage::Done(report));
                }
                Err(e) => {
                    state.install = Some(InstallStage::Done(install::Report {
                        ytdlp: None,
                        streamlink: Some(format!("Error: {e}")),
                        bin_dir: PathBuf::new(),
                    }));
                }
            }
            Task::none()
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    if state.install.is_some() {
        return install_view(state);
    }

    let mut col = Column::new()
        .push(
            text_input("Enter a URL or file path...", &state.source)
                .on_input(Message::SourceChanged)
                .padding(10)
                .size(16),
        )
        .push(controls_row(state))
        .push(status_view(state))
        .push(history_button(state))
        .spacing(12)
        .padding(20)
        .align_x(iced::Alignment::Center);

    if state.history_expanded {
        col = col.push(history_section(state));
    }

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.08, 0.08, 0.12))),
            ..Default::default()
        })
        .into()
}

pub fn theme(_state: &State) -> Theme {
    Theme::Dark
}

fn controls_row(state: &State) -> Element<'_, Message> {
    let quality_picker = pick_list(
        Quality::ALL.to_vec(),
        Some(state.quality),
        Message::QualityChanged,
    )
    .padding(8);

    let watch_btn: Element<'_, Message> = if state.can_watch() {
        widget::button(text("Watch").size(16))
            .on_press(Message::WatchPressed)
            .padding([10, 28])
            .style(|_, status| action_button_style(status))
            .into()
    } else {
        widget::button(text("Watch").size(16))
            .padding([10, 28])
            .style(|_, status| action_button_style(status))
            .into()
    };

    let download_btn: Element<'_, Message> = if state.can_download() {
        widget::button(text("Download").size(16))
            .on_press(Message::DownloadPressed)
            .padding([10, 28])
            .style(|_, status| action_button_style(status))
            .into()
    } else {
        widget::button(text("Download").size(16))
            .padding([10, 28])
            .style(|_, status| action_button_style(status))
            .into()
    };

    Row::new()
        .push(quality_picker)
        .push(watch_btn)
        .push(download_btn)
        .spacing(10)
        .into()
}

fn status_view(state: &State) -> Element<'_, Message> {
    match &state.status {
        Status::Resolving => {
            text("Resolving...").color(Color::from_rgb(0.5, 0.5, 0.5)).size(14).into()
        }
        Status::Ready => {
            text("Ready").color(Color::from_rgb(0.0, 0.7, 0.0)).size(14).into()
        }
        Status::Error(msg) => {
            text(msg).color(Color::from_rgb(1.0, 0.3, 0.3)).size(14).into()
        }
        Status::Idle => text("").into(),
    }
}

fn history_button(state: &State) -> Element<'_, Message> {
    if state.history_expanded {
        widget::button(text("History ▼").size(14))
            .on_press(Message::CloseHistory)
            .padding([6, 16])
            .into()
    } else {
        widget::button(text("History ▲").size(14))
            .on_press(Message::ToggleHistory)
            .padding([6, 16])
            .into()
    }
}

fn history_section(state: &State) -> Element<'_, Message> {
    if state.history.is_empty() {
        return Column::new()
            .push(text("No history yet.").size(14).color(Color::from_rgb(0.5, 0.5, 0.5)))
            .spacing(8)
            .into();
    }

    if state.show_clear_confirmation {
        return Column::new()
            .push(
                text("Are you sure you want to clear all history?").size(14),
            )
            .push(
                Row::new()
                    .push(
                        widget::button(text("Confirm").size(14))
                            .on_press(Message::ConfirmClearHistory)
                            .padding([8, 20])
                            .style(|_, status| confirm_button_style(status)),
                    )
                    .push(
                        widget::button(text("Cancel").size(14))
                            .on_press(Message::CancelClearHistory)
                            .padding([8, 20]),
                    )
                    .spacing(10),
            )
            .spacing(10)
            .padding(10)
            .into();
    }

    let entries: Vec<Element<'_, Message>> = state
        .history
        .iter()
        .rev()
        .enumerate()
        .map(|(i, entry)| {
            let row = Row::new()
                .push(text(&entry.title).width(Length::Fill).size(14))
                .spacing(5);

            widget::button(row)
                .on_press(Message::HistoryEntryClicked(entry.url.clone()))
                .padding([8, 12])
                .width(Length::Fill)
                .style(move |_, _| history_entry_style(i))
                .into()
        })
        .collect();

    Column::new()
        .push(
            text("History")
                .size(16)
                .color(Color::from_rgb(0.7, 0.7, 0.8)),
        )
        .push(
            scrollable(Column::with_children(entries).spacing(2))
                .width(Length::Fill)
                .height(250),
        )
        .push(
            widget::button(text("Clear history").size(14))
                .on_press(Message::ClearHistoryPressed)
                .padding([8, 20])
                .style(|_, status| subtle_button_style(status)),
        )
        .spacing(8)
        .into()
}

fn action_button_style(status: widget::button::Status) -> widget::button::Style {
    let base = widget::button::Style {
        background: Some(Background::Color(Color::from_rgb(0.78, 0.12, 0.12))),
        text_color: Color::WHITE,
        border: Border::default().rounded(6),
        ..Default::default()
    };

    match status {
        widget::button::Status::Active => base,
        widget::button::Status::Hovered => widget::button::Style {
            background: Some(Background::Color(Color::from_rgb(0.88, 0.18, 0.18))),
            ..base
        },
        widget::button::Status::Pressed => widget::button::Style {
            background: Some(Background::Color(Color::from_rgb(0.65, 0.08, 0.08))),
            ..base
        },
        widget::button::Status::Disabled => widget::button::Style {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.35))),
            text_color: Color::from_rgb(0.5, 0.5, 0.55),
            border: Border::default().rounded(6),
            ..Default::default()
        },
    }
}

fn history_entry_style(index: usize) -> widget::button::Style {
    let bg = if index % 2 == 0 {
        Color::from_rgb(0.12, 0.18, 0.35)
    } else {
        Color::from_rgb(0.08, 0.12, 0.25)
    };

    widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb(0.8, 0.8, 0.9),
        border: Border::default().rounded(4),
        ..Default::default()
    }
}

fn confirm_button_style(status: widget::button::Status) -> widget::button::Style {
    action_button_style(status)
}

fn subtle_button_style(status: widget::button::Status) -> widget::button::Style {
    let base = widget::button::Style {
        background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.2))),
        text_color: Color::from_rgb(0.7, 0.7, 0.8),
        border: Border::default().rounded(4),
        ..Default::default()
    };

    match status {
        widget::button::Status::Active => base,
        widget::button::Status::Hovered => widget::button::Style {
            background: Some(Background::Color(Color::from_rgb(0.22, 0.22, 0.28))),
            ..base
        },
        widget::button::Status::Pressed => widget::button::Style {
            background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.16))),
            ..base
        },
        widget::button::Status::Disabled => base,
    }
}

fn resolve_and_watch(url: &str, quality: &str, player: &str) -> Result<String, String> {
    if platform::is_youtube_url(url) {
        let title = ytdlp::resolve(url)
            .ok()
            .and_then(|i| i.title)
            .unwrap_or_default();
        if player == "mpv" {
            let format = ytdlp::format_for_quality(quality);
            platform::silent("mpv")
                .arg(format!("--ytdl-format={}", &format))
                .arg(url)
                .spawn()
                .map_err(|e| format!("Failed to launch mpv: {e}"))?;
        } else {
            let urls = ytdlp::stream_urls(url, "best")?;
            platform::launch_player(player, &urls)?;
        }
        Ok(title)
    } else {
        let info = streamlink::resolve(url)?;
        streamlink::play(url, quality, player).ok();
        Ok(info.title.unwrap_or_default())
    }
}

fn normalize_url(url: &str) -> String {
    if url.contains("://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

// ── Install prompt view ────────────────────────────────────────────────────

fn install_view(state: &State) -> Element<'_, Message> {
    let inner: Element<'_, Message> = match state.install.as_ref().unwrap() {
        InstallStage::Prompt => install_prompt_view(),
        InstallStage::Progress => install_progress_view(),
        InstallStage::Done(report) => install_done_view(report),
    };

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.08, 0.08, 0.12))),
            ..Default::default()
        })
        .into()
}

fn install_prompt_view() -> Element<'static, Message> {
    let missing: Vec<&str> = [("yt-dlp", ytdlp::exists()), ("streamlink", streamlink::exists())]
        .iter()
        .filter_map(|(name, ok)| if !ok { Some(*name) } else { None })
        .collect();

    let msg = if missing.is_empty() {
        "All tools are installed.".to_string()
    } else {
        format!("The following tools are required but not found:\n{}", missing.join(", "))
    };

    let content = Column::new()
        .push(text("Setup Required").size(20).color(Color::from_rgb(0.9, 0.9, 1.0)))
        .push(text("").size(8))
        .push(text(msg).size(14).color(Color::from_rgb(0.7, 0.7, 0.8)))
        .push(text("").size(12))
        .push(
            Row::new()
                .push(
                    widget::button(text("Install").size(16))
                        .on_press(Message::InstallAccept)
                        .padding([10, 28])
                        .style(|_, status| action_button_style(status)),
                )
                .push(
                    widget::button(text("Skip").size(16))
                        .on_press(Message::InstallDismiss)
                        .padding([10, 28])
                        .style(|_, status| subtle_button_style(status)),
                )
                .spacing(14)
                .align_y(iced::Alignment::Center),
        )
        .spacing(4)
        .align_x(iced::Alignment::Center);

    container(content)
        .padding(32)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.14, 0.14, 0.2))),
            border: Border::default().rounded(10),
            ..Default::default()
        })
        .into()
}

fn install_progress_view() -> Element<'static, Message> {
    let content = Column::new()
        .push(text("Installing...").size(20).color(Color::from_rgb(0.9, 0.9, 1.0)))
        .push(text("").size(12))
        .push(
            text("Downloading and installing missing tools.\nThis may take a moment.")
                .size(14)
                .color(Color::from_rgb(0.7, 0.7, 0.8)),
        )
        .spacing(4)
        .align_x(iced::Alignment::Center);

    container(content)
        .padding(32)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.14, 0.14, 0.2))),
            border: Border::default().rounded(10),
            ..Default::default()
        })
        .into()
}

fn install_done_view(report: &install::Report) -> Element<'static, Message> {
    let mut lines: Vec<String> = Vec::new();

    if let Some(ref r) = report.ytdlp {
        lines.push(format!("yt-dlp: {r}"));
    }
    if let Some(ref r) = report.streamlink {
        lines.push(format!("streamlink: {r}"));
    }

    let ok = ytdlp::exists() || streamlink::exists();
    let done_label: &str = if ok { "Done" } else { "Issues" };
    let summary: &str = if ok {
        "Installation complete."
    } else {
        "Some tools could not be installed."
    };

    let mut content = Column::new()
        .push(
            text(done_label)
                .size(20)
                .color(Color::from_rgb(0.9, 0.9, 1.0)),
        )
        .push(text("").size(8))
        .push(text(summary).size(14).color(Color::from_rgb(0.7, 0.7, 0.8)))
        .push(text("").size(6));

    for line in lines.drain(..) {
        let color = if line.contains("Error") || line.contains("Could not") {
            Color::from_rgb(1.0, 0.3, 0.3)
        } else {
            Color::from_rgb(0.3, 0.8, 0.3)
        };
        content = content.push(text(line).size(13).color(color));
    }

    if !report.bin_dir.as_os_str().is_empty() {
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            let dir = report.bin_dir.to_string_lossy().to_string();
            content = content
                .push(text("").size(4))
                .push(
                    text(format!("Tools installed in:\n{dir}"))
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.7)),
                );
        }
    }

    content = content
        .push(text("").size(12))
        .push(
            widget::button(text("Continue").size(16))
                .on_press(Message::InstallDismiss)
                .padding([10, 28])
                .style(|_, status| action_button_style(status)),
        )
        .spacing(4)
        .align_x(iced::Alignment::Center);

    container(content)
        .padding(32)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.14, 0.14, 0.2))),
            border: Border::default().rounded(10),
            ..Default::default()
        })
        .into()
}

async fn do_install() -> Result<install::Report, String> {
    let need_ytdlp = !ytdlp::exists();
    let need_streamlink = !streamlink::exists();

    let ytdlp = if need_ytdlp {
        Some(install::install_ytdlp().unwrap_or_else(|e| format!("Error: {e}")))
    } else {
        None
    };

    let streamlink = if need_streamlink {
        Some(install::install_streamlink().unwrap_or_else(|e| format!("Error: {e}")))
    } else {
        None
    };

    let bin_dir = install::bin_dir().unwrap_or_default();

    Ok(install::Report {
        ytdlp,
        streamlink,
        bin_dir,
    })
}
