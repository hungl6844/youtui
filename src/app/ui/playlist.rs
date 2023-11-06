use std::path::PathBuf;
use std::sync::Arc;
use std::{borrow::Cow, fmt::Debug};

use crate::app::server::downloader::SongProgressUpdateType;
use crate::app::view::BasicConstraint;
use crate::error::Result;
use crate::{
    app::{
        player::{self, Request, Response},
        server,
        structures::DownloadStatus,
    },
    core::send_or_error,
};
use crossterm::event::KeyCode;
use ratatui::{backend::Backend, layout::Rect, terminal::Frame};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::app::structures::Percentage;
use crate::app::view::draw::draw_table;
use crate::app::view::{Loadable, Scrollable, TableView};
use crate::app::{
    component::{
        actionhandler::{
            Action, ActionHandler, ActionProcessor, KeyHandler, KeyRouter, Keybind, TextHandler,
        },
        contextpane::ContextPane,
    },
    structures::{AlbumSongsList, ListSong, ListSongID, ListStatus, PlayState},
    taskmanager::TaskID,
    ui::{UIMessage, WindowContext},
    view::Drawable,
};

const SONGS_AHEAD_TO_BUFFER: usize = 3;
const VOL_TICK: u8 = 5;
const MUSIC_DIR: &str = "music/";

pub struct Playlist {
    pub list: AlbumSongsList,
    pub cur_played_secs: Option<f64>,
    pub play_status: PlayState,
    pub volume: Percentage,
    ui_tx: mpsc::Sender<UIMessage>,
    pub help_shown: bool,
    keybinds: Vec<Keybind<PlaylistAction>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlaylistAction {
    ViewBrowser,
    Down,
    Up,
    PageDown,
    PageUp,
    PlaySelected,
}

pub struct MusicCache {
    songs: Vec<PathBuf>,
}

impl MusicCache {
    fn cache_song(&mut self, song: Arc<Vec<u8>>, path: PathBuf) {
        let mut p = PathBuf::new();
        p.push(MUSIC_DIR);
        p.push(&path);
        self.songs.push(path);
        std::fs::write(p, &*song);
    }
    fn retrieve_song(&self, path: PathBuf) -> std::result::Result<Option<Vec<u8>>, std::io::Error> {
        if self.songs.contains(&path) {
            let mut p = PathBuf::new();
            p.push(MUSIC_DIR);
            p.push(&path);
            return std::fs::read(p).map(|v| Some(v));
        }
        Ok(None)
    }
}

impl Action for PlaylistAction {
    fn context(&self) -> Cow<str> {
        "Playlist".into()
    }
    fn describe(&self) -> Cow<str> {
        match self {
            PlaylistAction::ViewBrowser => "View Browser",
            PlaylistAction::Down => "Down",
            PlaylistAction::Up => "Up",
            PlaylistAction::PageDown => "Page Down",
            PlaylistAction::PageUp => "Page Up",
            PlaylistAction::PlaySelected => "Play Selected",
        }
        .into()
    }
}

impl ContextPane<PlaylistAction> for Playlist {}

impl KeyHandler<PlaylistAction> for Playlist {
    fn get_keybinds<'a>(
        &'a self,
    ) -> Box<
        dyn Iterator<Item = &'a crate::app::component::actionhandler::Keybind<PlaylistAction>> + 'a,
    > {
        Box::new(self.keybinds.iter())
    }
}
impl KeyRouter<PlaylistAction> for Playlist {
    fn get_all_keybinds<'a>(
        &'a self,
    ) -> Box<
        dyn Iterator<Item = &'a crate::app::component::actionhandler::Keybind<PlaylistAction>> + 'a,
    > {
        self.get_keybinds()
    }
}

impl ActionProcessor<PlaylistAction> for Playlist {}

impl TextHandler for Playlist {
    fn push_text(&mut self, _c: char) {}
    fn pop_text(&mut self) {}
    fn is_text_handling(&self) -> bool {
        false
    }
    fn take_text(&mut self) -> String {
        Default::default()
    }
    fn replace_text(&mut self, text: String) {}
}

impl Drawable for Playlist {
    fn draw_chunk<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        draw_table(f, self, chunk, true);
    }
}

impl Loadable for Playlist {
    fn is_loading(&self) -> bool {
        false
    }
}

impl Scrollable for Playlist {
    fn get_selected_item(&self) -> usize {
        self.list.get_selected_item()
    }
    fn increment_list(&mut self, amount: isize) {
        self.list.increment_list(amount)
    }

    fn get_offset(&self, height: usize) -> usize {
        // TODO
        0
    }
}

impl TableView for Playlist {
    fn get_title(&self) -> Cow<str> {
        format!("Local playlist - {} songs", self.list.list.len()).into()
    }
    fn get_layout(&self) -> &[BasicConstraint] {
        // Not perfect as this method doesn't know the size of the parent.
        // TODO: Change the get_layout function to something more appropriate.
        &[
            BasicConstraint::Length(6),
            BasicConstraint::Length(3),
            BasicConstraint::Percentage(Percentage(33)),
            BasicConstraint::Percentage(Percentage(33)),
            BasicConstraint::Percentage(Percentage(33)),
            BasicConstraint::Length(9),
            BasicConstraint::Length(4),
        ]
    }
    fn get_items(&self) -> Box<dyn ExactSizeIterator<Item = crate::app::view::TableItem> + '_> {
        Box::new(self.list.list.iter().map(|ls| ls.get_fields_iter()))
    }
    fn get_headings(&self) -> Box<(dyn Iterator<Item = &'static str> + 'static)> {
        Box::new(["", "#", "Artist", "Album", "Song", "Duration", "Year"].into_iter())
    }
}

impl ActionHandler<PlaylistAction> for Playlist {
    async fn handle_action(&mut self, action: &PlaylistAction) {
        match action {
            PlaylistAction::ViewBrowser => self.view_browser().await,
            PlaylistAction::Down => self.increment_list(1),
            PlaylistAction::Up => self.increment_list(-1),
            PlaylistAction::PageDown => self.increment_list(10),
            PlaylistAction::PageUp => self.increment_list(-10),
            PlaylistAction::PlaySelected => self.play_selected().await,
        }
    }
}

impl Playlist {
    pub fn new(ui_tx: mpsc::Sender<UIMessage>) -> Self {
        // This could fail, made to try send to avoid needing to change function signature to asynchronous. Should change.
        ui_tx
            .try_send(Request::GetVolume)
            .unwrap_or_else(|e| error!("Error <{e}> received sending Get Volume message"));
        Playlist {
            help_shown: false,
            ui_tx,
            volume: Percentage(50),
            play_status: PlayState::NotPlaying,
            list: Default::default(),
            cur_played_secs: None,
            keybinds: playlist_keybinds(),
        }
    }
    pub async fn handle_tick(&mut self) {
        self.check_song_progress().await;
        self.process_messages().await;
        // self.download_upcoming_songs().await;
    }
    pub async fn check_song_progress(&mut self) {
        // Ask player for a progress update.
        if let PlayState::Playing(id) = self.play_status {
            info!("Tick received - requesting song progress update");
            let _ = self.ui_tx.send(player::Request::GetProgress(id)).await;
        }
    }
    pub async fn handle_song_progress_update(
        &mut self,
        update: SongProgressUpdateType,
        id: ListSongID,
    ) {
        if !self.check_id_is_cur(id) {
            return;
        }
        tracing::info!("Task valid - updating song download status");
        match update {
            SongProgressUpdateType::Started => {
                if let Some(song) = self.list.list.iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Queued;
                    // while let Ok(_) = self.player_rx.try_recv() {}
                }
            }
            SongProgressUpdateType::Completed(song_buf) => {
                let fut = self
                    .get_mut_song_from_id(id)
                    .map(|s| {
                        s.download_status = DownloadStatus::Downloaded(Arc::new(song_buf));
                        s.id
                    })
                    .map(|id| async move { self.play_if_was_buffering(id).await });
                if let Some(f) = fut {
                    f.await
                }
            }
            SongProgressUpdateType::Error => {
                if let Some(song) = self.list.list.iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Failed;
                }
            }
            SongProgressUpdateType::Downloading(p) => {
                if let Some(song) = self.list.list.iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Downloading(p);
                }
            }
        }
    }
    pub fn handle_set_volume(&mut self, p: Percentage) {
        self.volume = p;
    }

    pub async fn handle_set_to_paused(&mut self, id: ListSongID) {
        // TODO: Check id
        if let PlayState::Playing(_) = self.play_status {
            self.play_status = PlayState::Paused(id)
        }
    }
    pub async fn handle_done_playing(&mut self, id: ListSongID) {
        self.play_next_or_finish(id).await;
    }
    pub async fn handle_set_to_playing(&mut self, id: ListSongID) {
        // TODO: Check id
        if let PlayState::Paused(_) = self.play_status {
            self.play_status = PlayState::Playing(id)
        }
    }
    pub async fn handle_set_to_stopped(&mut self) {
        self.play_status = PlayState::Stopped
    }
    pub async fn play_selected(&mut self) {
        let Some(index) = self.list.cur_selected else {
            return;
        };
        let Some(id) = self.get_id_from_index(index) else {
            return;
        };
        self.play_song_id(id).await;
    }
    pub async fn view_browser(&mut self) {
        send_or_error(
            &self.ui_tx,
            UIMessage::ChangeContext(WindowContext::Browser),
        )
        .await;
    }
    pub async fn handle_next(&mut self) {
        match self.play_status {
            PlayState::Playing(id) => {
                self.play_next_or_finish(id).await;
            }
            _ => (),
        }
    }
    pub async fn handle_previous(&mut self) {
        self.play_prev().await;
    }
    pub async fn handle_increase_volume(&mut self) {
        // Update the volume in the UI for immediate visual feedback - response will be delayed one tick.
        // NOTE: could cause some visual race conditions.
        self.volume.0 = self
            .volume
            .0
            .checked_add(VOL_TICK)
            .unwrap_or(100)
            .clamp(0, 100);
        send_or_error(&self.request_tx, Request::IncreaseVolume(VOL_TICK as i8)).await;
    }
    pub async fn handle_decrease_volume(&mut self) {
        // Update the volume in the UI for immediate visual feedback - response will be delayed one tick.
        // NOTE: could cause some visual race conditions.
        self.volume.0 = self
            .volume
            .0
            .checked_sub(VOL_TICK)
            .unwrap_or(0)
            .clamp(0, 100);
        send_or_error(&self.request_tx, Request::IncreaseVolume(-(VOL_TICK as i8))).await;
    }
    // Returns the ID of the first song added.
    pub fn push_song_list(&mut self, song_list: Vec<ListSong>) -> ListSongID {
        self.list.push_song_list(song_list)
    }
    pub fn push_clone_listsong(&mut self, song: &ListSong) -> ListSongID {
        // Are duplicate songs ok?
        self.list.push_clone_listsong(song)
    }
    pub async fn play_if_was_buffering(&mut self, id: ListSongID) {
        if let PlayState::Buffering(target_id) = self.play_status {
            if target_id == id {
                info!("playing");
                self.play_song_id(id).await;
            }
        }
    }
    // Ideally owned by list itself.
    pub async fn reset(&mut self) -> Result<()> {
        self.request_tx.send(Request::Stop).await?;
        self.list.state = ListStatus::New;
        // We can't reset the ID, we'll keep incrementing.
        // self.list.next_id = ListSongID(0);
        self.list.list.clear();
        self.list.cur_selected = None;
        self.cur_played_secs = None;
        // XXX: Also need to kill pending download tasks
        Ok(())
    }
    pub async fn play_song_id(&mut self, id: ListSongID) {
        // TODO: Stop currently playing song.
        self.download_upcoming_from_id(id).await;
        if let Some(song_index) = self.get_index_from_id(id) {
            if let DownloadStatus::Downloaded(pointer) = &self
                .list
                .list
                .get(song_index)
                .expect("Checked previously")
                .download_status
            {
                send_or_error(&self.request_tx, Request::PlaySong(pointer.clone(), id)).await;
                // send_or_error(&self.request_tx, Request::PlaySong(path.clone(), id)).await;
                self.play_status = PlayState::Playing(id);
            } else {
                self.play_status = PlayState::Buffering(id);
            }
        }
    }
    pub async fn download_song_if_exists(&mut self, id: ListSongID) {
        let Some(song_index) = self.get_index_from_id(id) else {
            return;
        };
        let song = self
            .list
            .list
            .get_mut(song_index)
            .expect("We got the index from the id, so song must exist");
        // Won't download if already downloaded, or downloading.
        if let DownloadStatus::Downloaded(_) = song.download_status {
            return;
        }
        if let DownloadStatus::Downloading(_) = song.download_status {
            return;
        }
        if let DownloadStatus::Queued = song.download_status {
            return;
        }
        send_or_error(
            &self.ui_tx,
            UIMessage::DownloadSong(song.raw.get_video_id().clone(), id),
        )
        .await;
        song.download_status = DownloadStatus::Queued;
    }
    pub async fn play_next_or_finish(&mut self, prev_id: ListSongID) {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play next, but not currently playing");
            }
            PlayState::Transitioning => {
                tracing::error!("Asked to play next, but between states. Should not be here!");
            }
            PlayState::Paused(id) | PlayState::Playing(id) | PlayState::Buffering(id) => {
                // Guard against duplicate message received.
                if id > &prev_id {
                    return;
                }
                let next_song_id = self
                    .get_index_from_id(*id)
                    .map(|i| i + 1)
                    .and_then(|i| self.get_id_from_index(i));
                match next_song_id {
                    Some(id) => {
                        self.play_song_id(id).await;
                    }
                    None => {
                        info!("No next song - finishing playback");
                        self.set_play_has_finished();
                    }
                }
            }
        }
    }
    pub async fn download_upcoming_from_id(&mut self, id: ListSongID) {
        // Won't download if already downloaded.
        let Some(song_index) = self.get_index_from_id(id) else {
            return;
        };
        let mut song_ids_list = Vec::new();
        song_ids_list.push(id);
        for i in 1..SONGS_AHEAD_TO_BUFFER {
            let next_id = self.list.list.get(song_index + i).map(|song| song.id);
            if let Some(id) = next_id {
                song_ids_list.push(id);
            }
        }
        for song_id in song_ids_list {
            self.download_song_if_exists(song_id).await;
        }
    }
    pub async fn play_prev(&mut self) {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play prev, but not currently playing");
            }
            PlayState::Transitioning => {
                tracing::error!("Asked to play prev, but between states. Should not be here!");
            }
            PlayState::Paused(id) | PlayState::Playing(id) | PlayState::Buffering(id) => {
                let prev_song_id = self
                    .get_index_from_id(*id)
                    .and_then(|i| i.checked_sub(1))
                    .and_then(|i| self.list.list.get(i))
                    .map(|i| i.id);
                info!("Next song id {:?}", prev_song_id);
                match prev_song_id {
                    Some(id) => {
                        self.play_song_id(id).await;
                    }
                    None => {
                        // TODO: Reset song to start if got here.
                        warn!("No previous song. Doing nothing")
                    }
                }
            }
        }
    }
    pub fn set_play_has_finished(&mut self) {
        self.play_status = self
            .play_status
            .take_whilst_transitioning()
            .transition_to_stopped();
    }
    pub fn update_song_progress(&mut self, new_play_time: f64) {
        self.cur_played_secs = Some(new_play_time);
    }
    pub async fn pauseplay(&mut self) {
        send_or_error(&self.ui_tx, UIMessage::PausePlay).await;
    }
    pub fn get_index_from_id(&self, id: ListSongID) -> Option<usize> {
        self.list.list.iter().position(|s| s.id == id)
    }
    pub fn get_id_from_index(&self, index: usize) -> Option<ListSongID> {
        self.list.list.get(index).map(|s| s.id)
    }
    pub fn get_mut_song_from_id(&mut self, id: ListSongID) -> Option<&mut ListSong> {
        self.list.list.iter_mut().find(|s| s.id == id)
    }
    pub fn get_song_from_id(&self, id: ListSongID) -> Option<&ListSong> {
        self.list.list.iter().find(|s| s.id == id)
    }
    pub fn check_id_is_cur(&self, check_id: ListSongID) -> bool {
        match self.play_status {
            // XXX: Should buffering be included?
            PlayState::Playing(id) | PlayState::Paused(id) => id == check_id,
            _ => false,
        }
    }

    pub fn cur_playing_index(&self) -> Option<usize> {
        match self.play_status {
            PlayState::Playing(id) | PlayState::Paused(id) => self.get_index_from_id(id),
            _ => None,
        }
    }
}

fn playlist_keybinds() -> Vec<Keybind<PlaylistAction>> {
    vec![
        Keybind::new_global_from_code(KeyCode::F(5), PlaylistAction::ViewBrowser),
        Keybind::new_from_code(KeyCode::Down, PlaylistAction::Down),
        Keybind::new_from_code(KeyCode::Up, PlaylistAction::Up),
        Keybind::new_from_code(KeyCode::PageDown, PlaylistAction::PageDown),
        Keybind::new_from_code(KeyCode::PageUp, PlaylistAction::PageUp),
        Keybind::new_from_code(KeyCode::Enter, PlaylistAction::PlaySelected),
    ]
}
