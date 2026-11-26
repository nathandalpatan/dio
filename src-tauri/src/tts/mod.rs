pub mod g2p;
pub mod kokoro;
pub mod model_store;

use serde::Serialize;
use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub accent: &'static str,
    #[serde(skip)]
    pub sha256: &'static str,
}

pub const VOICES: &[VoiceInfo] = &[
    VoiceInfo {
        id: "af_heart",
        name: "Heart",
        accent: "us",
        sha256: "d583ccff3cdca2f7fae535cb998ac07e9fcb90f09737b9a41fa2734ec44a8f0b",
    },
    VoiceInfo {
        id: "af_bella",
        name: "Bella",
        accent: "us",
        sha256: "f69d836209b78eb8c66e75e3cda491e26ea838a3674257e9d4e5703cbaf55c8b",
    },
    VoiceInfo {
        id: "am_michael",
        name: "Michael",
        accent: "us",
        sha256: "1d1f21dd8da39c30705cd4c75d039d265e9bc4a2a93ed09bc9e1b1225eb95ba1",
    },
    VoiceInfo {
        id: "am_fenrir",
        name: "Fenrir",
        accent: "us",
        sha256: "c27989f741f7ee34d273a39d8a595cc0837d35f5ced9a29b7cc162614616df43",
    },
    VoiceInfo {
        id: "bf_emma",
        name: "Emma",
        accent: "uk",
        sha256: "669fe0647f9dd04fcab92f1439a40eeb4c8b4ab1f82e4996fe3d918ce4a63b73",
    },
    VoiceInfo {
        id: "bm_george",
        name: "George",
        accent: "uk",
        sha256: "c4b235a4c1f2cd3b939fed08b899ce9385638b763f7b73a59616c4fc9bd6c9bc",
    },
];

pub fn voice(id: &str) -> Option<&'static VoiceInfo> {
    VOICES.iter().find(|voice| voice.id == id)
}

/// The preferred voice when it speaks `accent`, otherwise the first voice that does.
pub fn voice_for_accent(preferred: &str, accent: &str) -> &'static VoiceInfo {
    voice(preferred)
        .filter(|voice| voice.accent == accent)
        .or_else(|| VOICES.iter().find(|voice| voice.accent == accent))
        .unwrap_or(&VOICES[0])
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsState {
    /// disabled, notInstalled, downloading, loading, ready, synthesizing, playing, paused, error
    pub status: String,
    pub progress: f32,
    pub message: String,
    /// Which surface started playback: word, popup or reader.
    pub owner: String,
}

impl TtsState {
    pub fn active(&self) -> bool {
        matches!(
            self.status.as_str(),
            "loading" | "synthesizing" | "playing" | "paused"
        )
    }
}

pub trait Backend {
    fn synthesize(
        &mut self,
        text: &str,
        voice: &str,
        british: bool,
        speed: f32,
    ) -> Result<Vec<f32>, String>;
}

pub trait Output: Send + Sync {
    fn append(&self, samples: Vec<f32>);
    fn queued(&self) -> usize;
    fn finished(&self) -> bool;
    fn failed(&self) -> bool;
    fn pause(&self);
    fn play(&self);
    fn stop(&self);
    fn set_volume(&self, volume: f32);
}

type LoadBackend = Box<dyn FnMut() -> Result<Box<dyn Backend>, String> + Send>;
/// A playback handle plus a guard that keeps the device open on the worker thread.
pub type OpenedOutput = (Arc<dyn Output>, Box<dyn Any>);
type OpenOutput = Box<dyn FnMut() -> Result<OpenedOutput, String> + Send>;
type Notify = Box<dyn Fn(TtsState) + Send + Sync>;

pub struct Request {
    pub text: String,
    pub voice: String,
    pub british: bool,
    pub speed: f32,
    pub volume: f32,
    pub owner: String,
}

struct Job {
    generation: u64,
    request: Request,
}

/// Audio to synthesize ahead of time so a later request for it plays at once. Empty text only
/// loads the model, which otherwise adds most of a second to the first request.
pub struct Prepare {
    pub text: String,
    pub voice: String,
    pub british: bool,
    pub speed: f32,
}

/// Work for the speech thread. A speech job always runs before preparation.
#[derive(Default)]
struct Queue {
    job: Option<Job>,
    prepare: Option<Prepare>,
}

type Key = (String, String, bool, u32);
fn key(text: &str, voice: &str, british: bool, speed: f32) -> Key {
    (text.into(), voice.into(), british, speed.to_bits())
}
/// Prepared sentences kept for reuse, oldest dropped first.
const PREPARED: usize = 16;

const LOOKAHEAD: usize = 2;
const IDLE_UNLOAD: Duration = Duration::from_secs(10 * 60);

struct Shared {
    state: Mutex<TtsState>,
    generation: AtomicU64,
    pending: Mutex<Queue>,
    wake: Condvar,
    output: Mutex<Option<Arc<dyn Output>>>,
    paused: AtomicBool,
    installed: AtomicBool,
    enabled: AtomicBool,
    unload: AtomicBool,
    notify: Notify,
}

impl Shared {
    fn idle_status(&self) -> &'static str {
        if !self.enabled.load(Ordering::Acquire) {
            "disabled"
        } else if !self.installed.load(Ordering::Acquire) {
            "notInstalled"
        } else {
            "ready"
        }
    }

    fn set(&self, update: impl FnOnce(&mut TtsState)) {
        let next = {
            let mut state = self.state.lock().unwrap();
            update(&mut state);
            state.clone()
        };
        (self.notify)(next);
    }

    fn set_status(&self, status: &str, owner: &str, message: String) {
        self.set(|state| {
            *state = TtsState {
                status: status.into(),
                progress: 0.0,
                message,
                owner: owner.into(),
            }
        });
    }

    fn current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }

    /// Ends playback for `generation`, reporting `message` as an error when present.
    fn finish(&self, generation: u64, message: Option<String>) {
        if !self.current(generation) {
            return;
        }
        self.paused.store(false, Ordering::Release);
        match message {
            Some(message) => self.set_status("error", "", message),
            None => self.set_status(self.idle_status(), "", String::new()),
        }
    }
}

/// Owns one synthesis worker. A new request supersedes the previous one: its generation
/// number invalidates older chunks and the newest pending job replaces any queued job.
pub struct Tts {
    shared: Arc<Shared>,
}

impl Tts {
    pub fn new(
        installed: bool,
        enabled: bool,
        notify: Notify,
        mut load: LoadBackend,
        mut open: OpenOutput,
    ) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(TtsState::default()),
            generation: AtomicU64::new(0),
            pending: Mutex::new(Queue::default()),
            wake: Condvar::new(),
            output: Mutex::new(None),
            paused: AtomicBool::new(false),
            installed: AtomicBool::new(installed),
            enabled: AtomicBool::new(enabled),
            unload: AtomicBool::new(false),
            notify,
        });
        shared.state.lock().unwrap().status = shared.idle_status().into();
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("dio-tts".into())
            .spawn(move || {
                let mut backend: Option<Box<dyn Backend>> = None;
                let mut prepared: VecDeque<(Key, Vec<f32>)> = VecDeque::new();
                loop {
                    let (job, prepare) = {
                        let pending = worker.pending.lock().unwrap();
                        let (mut pending, timeout) = worker
                            .wake
                            .wait_timeout_while(pending, IDLE_UNLOAD, |queue| {
                                queue.job.is_none()
                                    && queue.prepare.is_none()
                                    && !worker.unload.load(Ordering::Acquire)
                            })
                            .unwrap();
                        if timeout.timed_out() || worker.unload.swap(false, Ordering::AcqRel) {
                            backend = None;
                        }
                        let job = pending.job.take();
                        let prepare = if job.is_none() {
                            pending.prepare.take()
                        } else {
                            None
                        };
                        (job, prepare)
                    };
                    if let Some(job) = job {
                        run(&worker, &mut backend, &mut load, &mut open, &prepared, job);
                    } else if let Some(prepare) = prepare {
                        prepare_audio(&mut backend, &mut load, &mut prepared, prepare);
                    }
                }
            })
            .expect("failed to start speech worker");
        Self { shared }
    }

    pub fn state(&self) -> TtsState {
        self.shared.state.lock().unwrap().clone()
    }

    pub fn available(&self) -> Result<(), String> {
        if !self.shared.enabled.load(Ordering::Acquire) {
            return Err("Speech is turned off in Settings.".into());
        }
        if !self.shared.installed.load(Ordering::Acquire) {
            return Err("Install the Kokoro voice in Settings to use speech.".into());
        }
        Ok(())
    }

    pub fn speak(&self, request: Request) -> Result<(), String> {
        self.available()?;
        let generation = self.interrupt();
        self.shared
            .set_status("loading", &request.owner, String::new());
        self.shared.pending.lock().unwrap().job = Some(Job {
            generation,
            request,
        });
        self.shared.wake.notify_one();
        Ok(())
    }

    /// Synthesizes `request` in the background so speaking it later starts at once. Does
    /// nothing when speech is unavailable, and never changes the reported state.
    pub fn prepare(&self, request: Prepare) {
        if self.available().is_err() {
            return;
        }
        self.shared.pending.lock().unwrap().prepare = Some(request);
        self.shared.wake.notify_one();
    }

    /// Loads the voice model ahead of the first request, if it is not loaded already.
    pub fn warm(&self) {
        let voice = &VOICES[0];
        self.prepare(Prepare {
            text: String::new(),
            voice: voice.id.into(),
            british: voice.accent == "uk",
            speed: 1.0,
        });
    }

    fn interrupt(&self) -> u64 {
        let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.shared.paused.store(false, Ordering::Release);
        self.shared.pending.lock().unwrap().job.take();
        if let Some(output) = self.shared.output.lock().unwrap().as_ref() {
            output.stop();
        }
        generation
    }

    /// Stops playback. With an owner, only that surface's playback is stopped.
    pub fn stop(&self, owner: Option<&str>) {
        let state = self.state();
        if owner.is_some_and(|owner| owner != state.owner) || !state.active() {
            return;
        }
        self.interrupt();
        self.shared
            .set_status(self.shared.idle_status(), "", String::new());
    }

    pub fn pause(&self) {
        let state = self.state();
        if !matches!(
            state.status.as_str(),
            "playing" | "synthesizing" | "loading"
        ) {
            return;
        }
        self.shared.paused.store(true, Ordering::Release);
        if let Some(output) = self.shared.output.lock().unwrap().as_ref() {
            output.pause();
        }
        self.shared.set(|state| state.status = "paused".into());
    }

    pub fn resume(&self) {
        if self.state().status != "paused" {
            return;
        }
        self.shared.paused.store(false, Ordering::Release);
        if let Some(output) = self.shared.output.lock().unwrap().as_ref() {
            output.play();
        }
        self.shared.set(|state| state.status = "playing".into());
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some(output) = self.shared.output.lock().unwrap().as_ref() {
            output.set_volume(volume);
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.shared.enabled.store(enabled, Ordering::Release);
        if !enabled {
            self.interrupt();
            self.shared.unload.store(true, Ordering::Release);
            self.shared.wake.notify_one();
        }
        if !self.state().active() {
            self.shared
                .set_status(self.shared.idle_status(), "", String::new());
        }
    }

    pub fn set_installed(&self, installed: bool, message: String) {
        self.shared.installed.store(installed, Ordering::Release);
        if !installed {
            self.interrupt();
            self.shared.unload.store(true, Ordering::Release);
            self.shared.wake.notify_one();
        }
        self.shared
            .set_status(self.shared.idle_status(), "", message);
    }

    pub fn set_downloading(&self, progress: f32) {
        self.shared.set(|state| {
            *state = TtsState {
                status: "downloading".into(),
                progress,
                message: String::new(),
                owner: String::new(),
            }
        });
    }
}

/// Splits text into sentence-sized pieces so the first audio starts quickly and each model
/// call stays small.
pub fn split_text(text: &str) -> Vec<String> {
    const TARGET: usize = 240;
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' || c == '\r' {
            if !current.trim().is_empty() {
                sentences.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        current.push(if c == '\t' { ' ' } else { c });
        let boundary = matches!(c, '.' | '!' | '?' | ';' | ':')
            && chars.peek().is_none_or(|next| next.is_whitespace());
        let long = current.chars().count() >= TARGET && c.is_whitespace();
        if boundary || long {
            sentences.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current);
    }
    sentences
        .into_iter()
        .map(|sentence| sentence.trim().to_owned())
        .filter(|sentence| sentence.chars().any(char::is_alphanumeric))
        .collect()
}

fn prepare_audio(
    backend: &mut Option<Box<dyn Backend>>,
    load: &mut LoadBackend,
    prepared: &mut VecDeque<(Key, Vec<f32>)>,
    request: Prepare,
) {
    if backend.is_none() {
        // A failure is reported by the next real request, which loads again.
        match load() {
            Ok(engine) => *backend = Some(engine),
            Err(_) => return,
        }
    }
    let Some(engine) = backend.as_mut() else {
        return;
    };
    for chunk in split_text(&request.text) {
        let id = key(&chunk, &request.voice, request.british, request.speed);
        if prepared.iter().any(|(existing, _)| *existing == id) {
            continue;
        }
        let Ok(samples) = engine.synthesize(&chunk, &request.voice, request.british, request.speed)
        else {
            return;
        };
        if prepared.len() >= PREPARED {
            prepared.pop_front();
        }
        prepared.push_back((id, samples));
    }
}

fn run(
    shared: &Shared,
    backend: &mut Option<Box<dyn Backend>>,
    load: &mut LoadBackend,
    open: &mut OpenOutput,
    prepared: &VecDeque<(Key, Vec<f32>)>,
    job: Job,
) {
    let Job {
        generation,
        request,
    } = job;
    if backend.is_none() {
        match load() {
            Ok(loaded) => *backend = Some(loaded),
            Err(error) => {
                shared.finish(generation, Some(error));
                return;
            }
        }
    }
    let Some(engine) = backend.as_mut() else {
        return;
    };
    if !shared.current(generation) {
        return;
    }
    shared.set(|state| {
        if state.status == "loading" {
            state.status = "synthesizing".into();
        }
    });
    let (output, device) = match open() {
        Ok(opened) => opened,
        Err(error) => {
            shared.finish(generation, Some(error));
            return;
        }
    };
    output.set_volume(request.volume);
    {
        let mut slot = shared.output.lock().unwrap();
        if !shared.current(generation) {
            return;
        }
        if shared.paused.load(Ordering::Acquire) {
            output.pause();
        }
        *slot = Some(output.clone());
    }
    let mut error = None;
    let mut started = false;
    'chunks: for chunk in split_text(&request.text) {
        while output.queued() >= LOOKAHEAD {
            if !shared.current(generation) || output.failed() {
                break 'chunks;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if !shared.current(generation) {
            break;
        }
        let id = key(&chunk, &request.voice, request.british, request.speed);
        let ready = prepared.iter().find(|(existing, _)| *existing == id);
        match ready.map_or_else(
            || engine.synthesize(&chunk, &request.voice, request.british, request.speed),
            |(_, samples)| Ok(samples.clone()),
        ) {
            Ok(samples) if samples.is_empty() => {}
            Ok(samples) => {
                if !shared.current(generation) {
                    break;
                }
                output.append(samples);
                if !started {
                    started = true;
                    shared.set(|state| {
                        if state.status == "synthesizing" {
                            state.status = "playing".into();
                        }
                    });
                }
            }
            Err(message) => {
                error = Some(message);
                break;
            }
        }
    }
    while error.is_none() && shared.current(generation) && !output.finished() {
        if output.failed() {
            error = Some("The audio device stopped responding.".into());
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    if error.is_some() {
        output.stop();
    }
    {
        let mut slot = shared.output.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &output))
        {
            *slot = None;
        }
    }
    drop(device);
    shared.finish(generation, error);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};

    #[derive(Default)]
    struct FakeOutput {
        queued: Mutex<Vec<Vec<f32>>>,
        played: Mutex<Vec<Vec<f32>>>,
        paused: AtomicBool,
        stopped: AtomicBool,
        failed: AtomicBool,
        volume: Mutex<f32>,
    }
    impl FakeOutput {
        /// Plays one queued chunk unless paused.
        fn tick(&self) {
            if !self.paused.load(Ordering::Acquire) {
                let mut queued = self.queued.lock().unwrap();
                if !queued.is_empty() {
                    let chunk = queued.remove(0);
                    self.played.lock().unwrap().push(chunk);
                }
            }
        }
    }
    impl Output for FakeOutput {
        fn append(&self, samples: Vec<f32>) {
            self.queued.lock().unwrap().push(samples);
        }
        fn queued(&self) -> usize {
            self.queued.lock().unwrap().len()
        }
        fn finished(&self) -> bool {
            self.queued.lock().unwrap().is_empty()
        }
        fn failed(&self) -> bool {
            self.failed.load(Ordering::Acquire)
        }
        fn pause(&self) {
            self.paused.store(true, Ordering::Release);
        }
        fn play(&self) {
            self.paused.store(false, Ordering::Release);
        }
        fn stop(&self) {
            self.stopped.store(true, Ordering::Release);
            self.queued.lock().unwrap().clear();
        }
        fn set_volume(&self, volume: f32) {
            *self.volume.lock().unwrap() = volume;
        }
    }

    struct FakeBackend {
        calls: Arc<Mutex<Vec<String>>>,
        delay: Duration,
    }
    impl Backend for FakeBackend {
        fn synthesize(
            &mut self,
            text: &str,
            voice: &str,
            british: bool,
            speed: f32,
        ) -> Result<Vec<f32>, String> {
            std::thread::sleep(self.delay);
            if text.contains("broken") {
                return Err("Speech synthesis failed.".into());
            }
            self.calls
                .lock()
                .unwrap()
                .push(format!("{voice}/{british}/{speed}:{text}"));
            Ok(vec![0.0; text.len()])
        }
    }

    struct Harness {
        tts: Tts,
        output: Arc<FakeOutput>,
        calls: Arc<Mutex<Vec<String>>>,
        states: Receiver<TtsState>,
        loads: Arc<AtomicU64>,
    }

    fn harness(load_error: Option<&'static str>, open_error: bool, delay: u64) -> Harness {
        let output = Arc::new(FakeOutput::default());
        let calls = Arc::new(Mutex::new(vec![]));
        let loads = Arc::new(AtomicU64::new(0));
        let (sender, states) = channel();
        let sender = Mutex::new(sender);
        let (fake_calls, fake_output, load_count) = (calls.clone(), output.clone(), loads.clone());
        let tts = Tts::new(
            true,
            true,
            Box::new(move |state| {
                let _ = sender.lock().unwrap().send(state);
            }),
            Box::new(move || {
                load_count.fetch_add(1, Ordering::AcqRel);
                match load_error {
                    Some(error) => Err(error.into()),
                    None => Ok(Box::new(FakeBackend {
                        calls: fake_calls.clone(),
                        delay: Duration::from_millis(delay),
                    }) as Box<dyn Backend>),
                }
            }),
            Box::new(move || {
                if open_error {
                    return Err("No audio output device is available.".into());
                }
                let output: Arc<dyn Output> = fake_output.clone();
                Ok((output, Box::new(()) as Box<dyn Any>))
            }),
        );
        Harness {
            tts,
            output,
            calls,
            states,
            loads,
        }
    }

    fn request(text: &str, owner: &str) -> Request {
        Request {
            text: text.into(),
            voice: "af_heart".into(),
            british: false,
            speed: 1.25,
            volume: 0.5,
            owner: owner.into(),
        }
    }

    fn wait_for(harness: &Harness, status: &str) -> TtsState {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            harness.output.tick();
            if let Ok(state) = harness.states.recv_timeout(Duration::from_millis(10)) {
                if state.status == status {
                    return state;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "never reached {status}"
            );
        }
    }

    #[test]
    fn speaks_sentences_in_order_and_returns_to_ready() {
        let harness = harness(None, false, 0);
        harness
            .tts
            .speak(request("One. Two! Three?", "reader"))
            .unwrap();
        assert_eq!(wait_for(&harness, "playing").owner, "reader");
        wait_for(&harness, "ready");
        assert_eq!(
            *harness.calls.lock().unwrap(),
            [
                "af_heart/false/1.25:One.",
                "af_heart/false/1.25:Two!",
                "af_heart/false/1.25:Three?"
            ]
        );
        assert_eq!(harness.output.played.lock().unwrap().len(), 3);
        assert_eq!(*harness.output.volume.lock().unwrap(), 0.5);
    }

    #[test]
    fn rapid_requests_keep_only_the_newest_and_reuse_the_model() {
        let harness = harness(None, false, 30);
        for index in 0..20 {
            harness
                .tts
                .speak(request(&format!("Word {index}."), "word"))
                .unwrap();
        }
        wait_for(&harness, "ready");
        let calls = harness.calls.lock().unwrap();
        assert_eq!(calls.last().unwrap(), "af_heart/false/1.25:Word 19.");
        assert!(calls.len() <= 2, "{calls:?}");
        assert_eq!(harness.loads.load(Ordering::Acquire), 1);
    }

    #[test]
    fn pause_resume_and_stop_follow_the_state_machine() {
        let harness = harness(None, false, 0);
        let long: String = (0..8).map(|index| format!("Sentence {index}. ")).collect();
        harness.tts.speak(request(&long, "popup")).unwrap();
        wait_for(&harness, "playing");
        harness.tts.pause();
        assert_eq!(harness.tts.state().status, "paused");
        assert!(harness.output.paused.load(Ordering::Acquire));
        std::thread::sleep(Duration::from_millis(100));
        assert!(harness.output.queued() <= LOOKAHEAD);
        harness.tts.resume();
        assert_eq!(harness.tts.state().status, "playing");
        harness.tts.stop(Some("reader"));
        assert_eq!(harness.tts.state().status, "playing");
        harness.tts.stop(Some("popup"));
        assert_eq!(harness.tts.state().status, "ready");
        assert!(harness.output.stopped.load(Ordering::Acquire));
        std::thread::sleep(Duration::from_millis(100));
        assert!(harness.calls.lock().unwrap().len() < 8);
        assert_eq!(harness.tts.state().status, "ready");
    }

    #[test]
    fn failures_are_reported_and_recoverable() {
        let missing = harness(Some("The voice model could not be loaded."), false, 0);
        missing.tts.speak(request("Hello.", "word")).unwrap();
        assert_eq!(
            wait_for(&missing, "error").message,
            "The voice model could not be loaded."
        );
        missing.tts.speak(request("Hello.", "word")).unwrap();
        wait_for(&missing, "error");
        assert_eq!(missing.loads.load(Ordering::Acquire), 2);

        let no_device = harness(None, true, 0);
        no_device.tts.speak(request("Hello.", "word")).unwrap();
        assert_eq!(
            wait_for(&no_device, "error").message,
            "No audio output device is available."
        );

        let broken = harness(None, false, 0);
        broken.tts.speak(request("Fine. broken.", "word")).unwrap();
        assert_eq!(
            wait_for(&broken, "error").message,
            "Speech synthesis failed."
        );
        broken.output.failed.store(true, Ordering::Release);
        broken.output.paused.store(true, Ordering::Release);
        broken.tts.speak(request("Fine.", "word")).unwrap();
        assert_eq!(
            wait_for(&broken, "error").message,
            "The audio device stopped responding."
        );
    }

    #[test]
    fn prepared_words_play_without_synthesizing_again() {
        let harness = harness(None, false, 0);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let calls = |count: usize| {
            while harness.calls.lock().unwrap().len() < count {
                assert!(std::time::Instant::now() < deadline, "never prepared");
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        harness.tts.warm();
        while harness.loads.load(Ordering::Acquire) == 0 {
            assert!(std::time::Instant::now() < deadline, "never loaded");
            std::thread::sleep(Duration::from_millis(5));
        }
        harness.tts.warm();
        harness.tts.prepare(Prepare {
            text: "serendipity".into(),
            voice: "af_heart".into(),
            british: false,
            speed: 1.25,
        });
        calls(1);
        assert_eq!(harness.tts.state().status, "ready");
        assert!(
            harness.states.try_recv().is_err(),
            "preparing changed the state"
        );
        harness.tts.speak(request("serendipity", "word")).unwrap();
        wait_for(&harness, "playing");
        wait_for(&harness, "ready");
        assert_eq!(
            *harness.calls.lock().unwrap(),
            ["af_heart/false/1.25:serendipity"]
        );
        assert_eq!(harness.loads.load(Ordering::Acquire), 1);
        assert_eq!(harness.output.played.lock().unwrap().len(), 1);
    }

    #[test]
    fn preparing_is_skipped_when_speech_is_unavailable() {
        let harness = harness(None, false, 0);
        harness.tts.set_enabled(false);
        harness.tts.warm();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(harness.loads.load(Ordering::Acquire), 0);
    }

    #[test]
    fn refuses_when_disabled_or_not_installed() {
        let harness = harness(None, false, 0);
        harness.tts.set_enabled(false);
        assert_eq!(harness.tts.state().status, "disabled");
        assert!(harness.tts.speak(request("Hi.", "word")).is_err());
        harness.tts.set_enabled(true);
        harness.tts.set_installed(false, String::new());
        assert_eq!(harness.tts.state().status, "notInstalled");
        assert!(harness.tts.speak(request("Hi.", "word")).is_err());
    }

    #[test]
    fn splits_text_into_bounded_sentences() {
        assert_eq!(
            split_text("Dr. Smith said: hi!\r\n\r\nNew line\tnow. 3.14 is pi"),
            ["Dr.", "Smith said:", "hi!", "New line now.", "3.14 is pi"]
        );
        let long = "word ".repeat(200);
        assert!(split_text(&long)
            .iter()
            .all(|piece| piece.chars().count() <= 245));
        assert!(split_text(" ... \n -- ").is_empty());
    }

    #[test]
    fn accent_voice_falls_back_to_a_matching_voice() {
        assert_eq!(voice_for_accent("am_michael", "us").id, "am_michael");
        assert_eq!(voice_for_accent("am_michael", "uk").id, "bf_emma");
        assert_eq!(voice_for_accent("bm_george", "uk").id, "bm_george");
    }
}
