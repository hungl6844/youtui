use self::statemanager::process_state_updates;
use self::taskmanager::TaskManager;
use super::appevent::{AppEvent, EventHandler};
use super::Result;
use crate::app::server::api::Api;
use crate::config::Config;
use crate::{get_data_dir, RuntimeInfo};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::process;
use std::{io, sync::Arc};
use tracing::info;
use tracing_subscriber::prelude::*;
use ui::YoutuiWindow;

mod component;
mod musiccache;
mod server;
mod statemanager;
mod structures;
mod taskmanager;
mod ui;
mod view;

const EVENT_CHANNEL_SIZE: usize = 256;
const LOG_FILE_NAME: &str = "debug.log";

pub struct Youtui {
    event_handler: EventHandler,
    window_state: YoutuiWindow,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    task_manager: TaskManager,
}

impl Youtui {
    pub fn new(rt: RuntimeInfo) -> Result<Youtui> {
        let RuntimeInfo {
            debug,
            config,
            api_key,
        } = rt;
        // TODO: Handle errors
        // Setup tracing and link to tui_logger.
        let tui_logger_layer = tui_logger::tracing_subscriber_layer();
        // Hold off implementing log file until dirs improved.
        // let log_file = std::fs::File::create(get_data_dir()?.join(LOG_FILE_NAME))?;
        // let log_file_layer = tracing_subscriber::fmt::layer().with_writer(Arc::new(log_file));
        // TODO: Confirm if this filter is correct.
        let context_layer =
            tracing_subscriber::filter::Targets::new().with_target("youtui", tracing::Level::DEBUG);
        tracing_subscriber::registry()
            .with(
                tui_logger_layer, // Hold off from implementing log file until dirs support improved.
                                  // .and_then(log_file_layer)
            )
            .with(context_layer)
            .init();
        info!("Starting");
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        // Ensure clean return to shell if panic.
        std::panic::set_hook(Box::new(|panic_info| {
            destruct_terminal();
            println!("{}", panic_info);
        }));
        let task_manager = taskmanager::TaskManager::new(api_key);
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let event_handler = EventHandler::new(EVENT_CHANNEL_SIZE)?;
        let window_state = YoutuiWindow::new(task_manager.get_sender_clone().clone());
        Ok(Youtui {
            terminal,
            event_handler,
            window_state,
            task_manager,
        })
    }
    pub async fn run(&mut self) -> Result<()> {
        while self.window_state.get_status() == &ui::AppStatus::Running {
            // Get the events from the event_handler and process them.
            let msg = self.event_handler.next().await;
            self.process_message(msg).await;
            // If any requests are in the queue, queue up the tasks on the server.
            self.queue_server_tasks().await;
            // Get the state update events from the task manager and process them.
            let state_updates = self.task_manager.process_messages();
            process_state_updates(&mut self.window_state, state_updates).await;
            // Write to terminal, using UI state as the input
            // We draw after handling the event, as the event could be a keypress we want to instantly react to.
            // TODO: Error handling
            self.terminal.draw(|f| {
                ui::draw::draw_app(f, &self.window_state);
            })?;
        }
        Ok(())
    }
    async fn process_message(&mut self, msg: Option<AppEvent>) {
        // TODO: Handle closed channel
        match msg {
            Some(AppEvent::QuitSignal) => self.window_state.set_status(ui::AppStatus::Exiting),
            Some(AppEvent::Crossterm(e)) => self.window_state.handle_event(e).await,
            // XXX: Should be try_poll or similar? Poll the Future but don't await it?
            Some(AppEvent::Tick) => self.window_state.handle_tick().await,
            None => panic!("Channel closed"),
        }
    }
    async fn queue_server_tasks(&mut self) {
        self.task_manager.process_requests().await;
    }
}

fn destruct_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    execute!(io::stdout(), crossterm::cursor::Show)?;
    Ok(())
}
