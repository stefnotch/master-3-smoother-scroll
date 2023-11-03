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

    // 120 is the Windows hardcoded number of ticks per normal wheel revolution
    let handler = EventHandler::new(EventHandlerConfig {
        min_delta_size: 4.0 / 120.0,
        min_smoothed_delta_size: 2.0 / 120.0,
        time_constant: 80.0,
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
    /// How far does the smoothed mouse wheel need to be moved to be considered a scroll event?
    /// Should be smaller than the min_delta_size.
    min_smoothed_delta_size: f32,

    // How far does the mouse wheel need to be moved to be considered a scroll event?
    min_delta_size: f32,

    /// How many milliseconds it takes until the smoothed signal reaches 63.2% of it's real value.
    time_constant: f32,
}

struct EventHandler {
    last_scroll: Arc<Mutex<ScrollWithTimestamp>>,
    last_smoothed_scroll: Arc<Mutex<ScrollWithTimestamp>>,
    // TODO:
    _dropped_deltas: Arc<Mutex<(f32, f32)>>,
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
            last_smoothed_scroll: Arc::new(Mutex::new(Default::default())),
            _dropped_deltas: Arc::new(Mutex::new((0.0, 0.0))),
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

            if timestamp > last_delta.timestamp {
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
                if delta_x.abs() >= self.config.min_delta_size
                    && delta_y.abs() >= self.config.min_delta_size
                {
                    return true;
                } else {
                    return false;
                }
            }
        };

        let sign_changed = (delta_x.signum() != last_delta.delta_x.signum())
            || (delta_y.signum() != last_delta.delta_y.signum());

        // We compute an average scroll step (the mouse can sometimes randomly report a slightly higher step, and we wanna get rid of that)
        // See also https://en.wikipedia.org/wiki/Exponential_smoothing
        let alpha = 1.0 - f32::exp(-(duration.as_millis() as f32) / self.config.time_constant);
        let alpha = alpha.clamp(0.0, 1.0);
        let alpha = if sign_changed { 1.0 } else { alpha };
        let smoothed_delta = {
            let mut last_smoothed_delta_mutex = self.last_smoothed_scroll.lock().unwrap();
            let smoothed_delta = ScrollWithTimestamp {
                delta_x: delta_x * alpha + last_smoothed_delta_mutex.delta_x * (1.0 - alpha),
                delta_y: delta_y * alpha + last_smoothed_delta_mutex.delta_y * (1.0 - alpha),
                timestamp,
            };
            *last_smoothed_delta_mutex = smoothed_delta.clone();
            smoothed_delta
        };

        // If the sign changes, we want to keep the event
        if sign_changed {
            return true;
        }

        // If the delta is too small, we don't want to keep the event
        return smoothed_delta.delta_x.abs() >= self.config.min_smoothed_delta_size
            || smoothed_delta.delta_y.abs() >= self.config.min_smoothed_delta_size
            || delta_x.abs() >= self.config.min_delta_size
            || delta_y.abs() >= self.config.min_delta_size;
    }
}
