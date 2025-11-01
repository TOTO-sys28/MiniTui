use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::ipc::PlaybackState;

use minimp3::{Decoder as Mp3Decoder, Frame};

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackChanged(()),
    StateChanged(()),
}

struct Mp3Source<R: std::io::Read> {
    decoder: Mp3Decoder<R>,
    current_frame: Option<Frame>,
    frame_pos: usize,
}

impl<R: std::io::Read> Mp3Source<R> {
    fn new(reader: R) -> Self {
        Self {
            decoder: Mp3Decoder::new(reader),
            current_frame: None,
            frame_pos: 0,
        }
    }
}

impl<R: std::io::Read> rodio::Source for Mp3Source<R> {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.current_frame.as_ref().map(|f| f.channels as u16).unwrap_or(2)
    }

    fn sample_rate(&self) -> u32 {
        self.current_frame.as_ref().map(|f| f.sample_rate as u32).unwrap_or(44100)
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl<R: std::io::Read> Iterator for Mp3Source<R> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        loop {
            if let Some(frame) = &self.current_frame {
                if self.frame_pos < frame.data.len() {
                    let sample = frame.data[self.frame_pos];
                    self.frame_pos += 1;
                    return Some(sample);
                } else {
                    self.current_frame = None;
                    self.frame_pos = 0;
                }
            }
            match self.decoder.next_frame() {
                Ok(frame) => {
                    self.current_frame = Some(frame);
                }
                Err(_) => return None,
            }
        }
    }
}

pub struct Player {
    sink: Arc<Mutex<Sink>>,
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    current_track: Arc<Mutex<Option<String>>>,
    state: Arc<Mutex<PlaybackState>>,
    volume: Arc<Mutex<u8>>,
    duration: Arc<Mutex<f64>>,
    start_time: Arc<Mutex<Option<std::time::Instant>>>,
    paused_position: Arc<Mutex<f64>>,
    event_tx: mpsc::UnboundedSender<PlayerEvent>,
}

impl Player {
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<PlayerEvent>)> {
        let (stream, stream_handle) = OutputStream::try_default()
            .context("Failed to create audio output stream")?;
        
        let sink = Sink::try_new(&stream_handle)
            .context("Failed to create audio sink")?;
        
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let player = Self {
            sink: Arc::new(Mutex::new(sink)),
            _stream: stream,
            _stream_handle: stream_handle,
            current_track: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            volume: Arc::new(Mutex::new(70)),
            duration: Arc::new(Mutex::new(0.0)),
            start_time: Arc::new(Mutex::new(None)),
            paused_position: Arc::new(Mutex::new(0.0)),
            event_tx,
        };
        
        // Set initial volume
        player.sink.lock().unwrap().set_volume(0.7);
        
        Ok((player, event_rx))
    }

    pub fn load_track(&self, path: String) -> Result<()> {
        let is_mp3 = path.to_lowercase().ends_with(".mp3");

        let data = std::fs::read(&path)
            .context(format!("Failed to read audio file: {}", path))?;
        let cursor = std::io::Cursor::new(data);

        let (source, duration) = if is_mp3 {
            let mp3_source = Mp3Source::new(cursor);
            // For MP3, try to get duration by decoding a bit, but for simplicity, use 0.0
            let duration = 0.0; // TODO: estimate duration for MP3
            (Box::new(mp3_source) as Box<dyn Source<Item = i16> + Send>, duration)
        } else {
            let source = Decoder::new(cursor)
                .context("Failed to decode audio file")?;
            let duration = source.total_duration()
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            (Box::new(source) as Box<dyn Source<Item = i16> + Send>, duration)
        };

        *self.duration.lock().unwrap() = duration;

        // Clear current sink and create new one
        let sink = self.sink.lock().unwrap();
        sink.stop();

        // Load new track
        sink.append(source);
        sink.play(); // Ensure playback starts

        *self.start_time.lock().unwrap() = Some(std::time::Instant::now());
        *self.paused_position.lock().unwrap() = 0.0;

        *self.current_track.lock().unwrap() = Some(path.clone());
        *self.state.lock().unwrap() = PlaybackState::Playing;

        let _ = self.event_tx.send(PlayerEvent::TrackChanged(()));
        let _ = self.event_tx.send(PlayerEvent::StateChanged(()));

        Ok(())
    }

    pub fn play(&self) -> Result<()> {
        let sink = self.sink.lock().unwrap();

        // Always try to play if there's a current track
        if self.current_track.lock().unwrap().is_some() {
            sink.play();
            *self.start_time.lock().unwrap() = Some(std::time::Instant::now());
            *self.state.lock().unwrap() = PlaybackState::Playing;
            let _ = self.event_tx.send(PlayerEvent::StateChanged(()));
        }

        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        let sink = self.sink.lock().unwrap();
        
        if !sink.is_paused() {
            sink.pause();
            let elapsed = self.start_time.lock().unwrap().take().map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
            *self.paused_position.lock().unwrap() += elapsed;
            *self.state.lock().unwrap() = PlaybackState::Paused;
            let _ = self.event_tx.send(PlayerEvent::StateChanged(()));
        }
        
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let sink = self.sink.lock().unwrap();
        sink.stop();

        *self.start_time.lock().unwrap() = None;
        *self.paused_position.lock().unwrap() = 0.0;

        *self.current_track.lock().unwrap() = None;
        *self.state.lock().unwrap() = PlaybackState::Stopped;

        let _ = self.event_tx.send(PlayerEvent::StateChanged(()));
        
        Ok(())
    }

    pub fn set_volume(&self, level: u8) -> Result<()> {
        let level = level.min(100);
        let volume = level as f32 / 100.0;
        
        let sink = self.sink.lock().unwrap();
        sink.set_volume(volume);
        
        *self.volume.lock().unwrap() = level;
        
        Ok(())
    }

    pub fn get_volume(&self) -> u8 {
        *self.volume.lock().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.sink.lock().unwrap().empty()
    }

    pub fn get_state(&self) -> PlaybackState {
        self.state.lock().unwrap().clone()
    }

    pub fn get_current_track(&self) -> Option<String> {
        self.current_track.lock().unwrap().clone()
    }

    pub fn get_duration(&self) -> f64 {
        *self.duration.lock().unwrap()
    }

    pub fn get_position(&self) -> f64 {
        let paused = *self.paused_position.lock().unwrap();
        let elapsed = self.start_time.lock().unwrap().as_ref().map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        paused + elapsed
    }
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            sink: Arc::clone(&self.sink),
            _stream: OutputStream::try_default().unwrap().0,
            _stream_handle: OutputStream::try_default().unwrap().1,
            current_track: Arc::clone(&self.current_track),
            state: Arc::clone(&self.state),
            volume: Arc::clone(&self.volume),
            duration: Arc::clone(&self.duration),
            start_time: Arc::new(Mutex::new(None)),
            paused_position: Arc::new(Mutex::new(0.0)),
            event_tx: self.event_tx.clone(),
        }
    }
}
