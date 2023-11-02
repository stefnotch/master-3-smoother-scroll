#![feature(unboxed_closures, fn_traits)]

use rdev::{grab, Event, EventType, EventTypes, MouseScrollDelta};
use std::{
    sync::{Arc, Mutex},
    time::{self, Duration},
};

fn main() {
    // This will block.
    let handler = EventHandler::new();
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
    last_delta: Arc<Mutex<MouseScrollDeltaWithTimestamp>>,
}

#[derive(Clone, Debug)]
struct MouseScrollDeltaWithTimestamp {
    delta_x: f32,
    delta_y: f32,
    timestamp: time::SystemTime,
}

impl Default for MouseScrollDeltaWithTimestamp {
    fn default() -> Self {
        MouseScrollDeltaWithTimestamp {
            delta_x: 0.0,
            delta_y: 0.0,
            timestamp: time::SystemTime::UNIX_EPOCH,
        }
    }
}

impl EventHandler {
    pub fn new() -> Self {
        EventHandler {
            last_delta: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn callback(&self, event: Event) -> Option<Event> {
        match event.event_type {
            EventType::Wheel(MouseScrollDelta::LineDelta(delta_x, delta_y)) => {
                // Add new event
                let last_delta = {
                    let mut last_delta_mutex = self.last_delta.lock().unwrap();
                    let last_delta = last_delta_mutex.clone();
                    *last_delta_mutex = MouseScrollDeltaWithTimestamp {
                        delta_x,
                        delta_y,
                        timestamp: time::SystemTime::now(),
                    };
                    last_delta
                };

                let duration = time::SystemTime::now().duration_since(last_delta.timestamp);
                match duration {
                    Ok(_) => {}
                    Err(_) => {
                        eprintln!("Error: durations are whack!");
                    }
                };

                let duration_threshold = Duration::from_millis(1000);
                let (speed_x, speed_y) = duration
                    .map(|duration| {
                        if duration <= duration_threshold {
                            let speed_x = (delta_x) / duration.as_millis() as f32;
                            let speed_y = (delta_y) / duration.as_millis() as f32;
                            (speed_x, speed_y)
                        } else {
                            let speed_x = (delta_x - 0.0) / duration_threshold.as_millis() as f32;
                            let speed_y = (delta_y - 0.0) / duration_threshold.as_millis() as f32;
                            (speed_x, speed_y)
                        }
                    })
                    .unwrap_or((0.0, 0.0));
                // println!("Speed: {} {}", speed_x, speed_y);

                let min_speed = 0.0005;
                if speed_x.abs() < min_speed && speed_y.abs() < min_speed {
                    None
                } else {
                    Some(event)
                }
            }
            _ => Some(event),
        }
    }
}
