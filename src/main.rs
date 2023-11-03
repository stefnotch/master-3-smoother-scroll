use rdev::{grab, Event, EventType, EventTypes, MouseScrollDelta};
use std::{
    sync::{Arc, Mutex},
    time::{self},
};

fn main() {
    // 120 is the Windows hardcoded number of ticks per normal wheel revolution
    let min_delta_size = 2.0 / 120.0;
    let handler = EventHandler::new(min_delta_size, 100.0);
    let callback = move |event: Event| handler.callback(event);
    if let Err(error) = grab(
        EventTypes {
            keyboard: false,
            mouse: true,
        },
        callback,
    ) {
        println!("Error: {:?}", error)
    }
}

struct EventHandler {
    last_scroll: Arc<Mutex<ScrollWithTimestamp>>,
    last_smoothed_scroll: Arc<Mutex<ScrollWithTimestamp>>,
    min_delta_size: f32,
    /// How many milliseconds it takes until the smoothed signal reaches 63.2% of it's real value.
    time_constant: f32,

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
    pub fn new(min_delta_size: f32, time_constant: f32) -> Self {
        EventHandler {
            last_scroll: Arc::new(Mutex::new(Default::default())),
            last_smoothed_scroll: Arc::new(Mutex::new(Default::default())),
            min_delta_size,
            time_constant,
            _start_time: time::SystemTime::now(),
        }
    }

    pub fn callback(&self, event: Event) -> Option<Event> {
        match event.event_type {
            EventType::Wheel(MouseScrollDelta::LineDelta(delta_x, delta_y)) => {
                // Add new event
                let last_delta = {
                    let mut last_delta_mutex = self.last_scroll.lock().unwrap();
                    let last_delta = last_delta_mutex.clone();

                    if event.time > last_delta.timestamp {
                        *last_delta_mutex = ScrollWithTimestamp {
                            delta_x,
                            delta_y,
                            timestamp: event.time,
                        };
                    }
                    last_delta
                };

                let duration = match time::SystemTime::now().duration_since(last_delta.timestamp) {
                    Ok(duration) => duration,
                    Err(_) => {
                        // Shouldn't really happen. I'll just shoddily fake it then.
                        if delta_x.abs() >= self.min_delta_size
                            && delta_y.abs() >= self.min_delta_size
                        {
                            return Some(event);
                        } else {
                            return None;
                        }
                    }
                };

                let sign_changed = (delta_x.signum() != last_delta.delta_x.signum())
                    || (delta_y.signum() != last_delta.delta_y.signum());

                // We compute an average scroll step (the mouse can sometimes randomly report a slightly higher step, and we wanna get rid of that)
                let alpha = 1.0 - f32::exp(-(duration.as_millis() as f32) / self.time_constant);
                let alpha = alpha.clamp(0.0, 1.0);
                let alpha = if sign_changed { 1.0 } else { alpha };
                let smoothed_delta = {
                    let mut last_smoothed_delta_mutex = self.last_smoothed_scroll.lock().unwrap();
                    let smoothed_delta = ScrollWithTimestamp {
                        delta_x: delta_x * alpha
                            + last_smoothed_delta_mutex.delta_x * (1.0 - alpha),
                        delta_y: delta_y * alpha
                            + last_smoothed_delta_mutex.delta_y * (1.0 - alpha),
                        timestamp: event.time,
                    };
                    *last_smoothed_delta_mutex = smoothed_delta.clone();
                    smoothed_delta
                };

                // If the sign changes, we want to keep the event
                if sign_changed {
                    return Some(event);
                }

                // If the delta is too small, we don't want to keep the event
                if smoothed_delta.delta_x.abs() < self.min_delta_size
                    && smoothed_delta.delta_y.abs() < self.min_delta_size
                {
                    return None;
                } else {
                    return Some(event);
                }
            }
            _ => Some(event),
        }
    }
}
