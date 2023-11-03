#![windows_subsystem = "windows"]
mod app_config;

use rdev::{grab, Event, EventType, EventTypes, MouseScrollDelta};
use std::{
    sync::{Arc, Mutex},
    time::{self},
};
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

use crate::app_config::read_config;

fn initialize_logging() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logging()?;
    info!("Starting application");

    let config = read_config()?;
    if config.log_to_file {
        // Configure a file logger if log_to_file is enabled
        let file_appender = tracing_appender::rolling::daily("logs", "app.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let file_subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_writer(non_blocking)
            .with_max_level(tracing::Level::INFO)
            .finish();
        tracing::subscriber::set_global_default(file_subscriber)?;
        info!("Logging to file is enabled");
    }

    info!("Config: {:?}", config);

    // 1. Plot the scroll values
    // 2. Plot the speed values
    // 3. https://docs.google.com/spreadsheets/d/1irAZETTmwKNsD2Ho1e1_RrDXjAiplB_sUgW0JJKhyBM/edit#gid=0
    // 4. Oh, so that's why the speed limiting works so well
    let handler = EventHandler::new(EventHandlerConfig {
        min_speed: 0.1,
        force_start_distance: 3.9 / 120.0,
    });
    let callback = move |event: Event| handler.callback(event);
    if let Err(error) = grab(
        EventTypes {
            keyboard: false,
            mouse: true,
        },
        callback,
    ) {
        error!("Error: {:?}", error);
    }

    Ok(())
}

struct EventHandlerConfig {
    min_speed: f32,
    force_start_distance: f32,
}

struct EventHandler {
    last_scroll: Arc<Mutex<ScrollWithTimestamp>>,
    config: EventHandlerConfig,
    // For plotting the data
    _start_time: time::SystemTime,
}

#[derive(Clone, Debug)]
struct ScrollWithTimestamp {
    delta_x: f32,
    delta_y: f32,
    timestamp: time::SystemTime,
}

impl Default for ScrollWithTimestamp {
    fn default() -> Self {
        ScrollWithTimestamp {
            delta_x: 0.0,
            delta_y: 0.0,
            timestamp: time::SystemTime::UNIX_EPOCH,
        }
    }
}

impl EventHandler {
    pub fn new(config: EventHandlerConfig) -> Self {
        EventHandler {
            last_scroll: Arc::new(Mutex::new(Default::default())),
            config,
            _start_time: time::SystemTime::now(),
        }
    }

    pub fn callback(&self, event: Event) -> Option<Event> {
        match event.event_type {
            EventType::Wheel(MouseScrollDelta::LineDelta(delta_x, delta_y)) => {
                let timestamp = event.time;
                let should_keep_event = self.handle_mouse_scroll(timestamp, delta_x, delta_y);
                if should_keep_event {
                    Some(event)
                } else {
                    None
                }
            }
            _ => Some(event),
        }
    }

    fn handle_mouse_scroll(&self, timestamp: time::SystemTime, delta_x: f32, delta_y: f32) -> bool {
        // Add new event
        let last_delta = {
            let mut last_delta_mutex = self.last_scroll.lock().unwrap();
            let last_delta = last_delta_mutex.clone();

            if timestamp >= last_delta.timestamp {
                *last_delta_mutex = ScrollWithTimestamp {
                    delta_x,
                    delta_y,
                    timestamp,
                };
            }
            last_delta
        };

        let duration = match time::SystemTime::now().duration_since(last_delta.timestamp) {
            Ok(duration) => duration,
            Err(_) => {
                // Shouldn't really happen. I'll just shoddily fake it then.
                return false;
            }
        };

        let sign_changed = (delta_x.signum() != last_delta.delta_x.signum())
            || (delta_y.signum() != last_delta.delta_y.signum());

        let speed_x = delta_x / (duration.as_millis() as f32);
        let speed_y = delta_y / (duration.as_millis() as f32);

        // If the sign changes, we want to keep the event
        if sign_changed {
            return true;
        }

        // If the delta is too small, we don't want to keep the event
        return speed_x.abs() >= self.config.min_speed
            || speed_y.abs() >= self.config.min_speed
            || delta_x.abs() >= self.config.force_start_distance
            || delta_y.abs() >= self.config.force_start_distance;
    }
}
