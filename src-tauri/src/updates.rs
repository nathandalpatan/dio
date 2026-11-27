//! Update checks, downloads and installation, in one of three modes: auto, notify or manual.
//!
//! `Machine` holds what Settings → Updates shows and the record kept in `update.json`, without any
//! I/O, so every transition is testable. `Updates` runs it against a `Source` (the Tauri updater in
//! production) and reports through a `Sink` (window events and Windows notifications).

use semver::Version;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::async_runtime::{self, JoinHandle};

pub const DAY: i64 = 24 * 60 * 60;
/// Seconds without keyboard or mouse input before a downloaded update may restart DIO.
pub const IDLE_BEFORE_INSTALL: u64 = 10 * 60;
/// Delay before the first automatic check, so updates never slow down startup.
pub const FIRST_CHECK: Duration = Duration::from_secs(2 * 60);
/// How often the scheduler looks at its local state. It only goes online once a check is due.
pub const TICK: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    Ready,
    RestartRequired,
    Installing,
    Updated,
    Failed,
}

/// The step that failed, so "Try again" repeats it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    #[serde(rename = "")]
    None,
    Check,
    Download,
    Install,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub status: Status,
    pub current: String,
    pub version: String,
    pub notes: Vec<String>,
    pub progress: f64,
    pub error: String,
    pub retry: Step,
    pub last_checked: Option<i64>,
}

/// What survives a restart, in `update.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Record {
    pub last_checked: Option<i64>,
    /// The update handed to the installer, compared with the running version at the next start.
    pub pending: Option<Pending>,
    /// The last version whose "DIO has been updated" notification was shown.
    pub notified_version: Option<String>,
    /// The last version an "update available" notification announced.
    pub announced: Option<String>,
    pub remind: Option<Remind>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub version: String,
    pub summary: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Remind {
    pub version: String,
    pub until: i64,
}

/// A newer release, as described by the update manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub summary: String,
    pub notes: Vec<String>,
}

/// Most bullet points shown in the "What's new" list.
const NOTES_SHOWN: usize = 5;

impl Release {
    /// Reads the summary paragraph and the bullet points from notes written by `tools/changelog.mjs`.
    pub fn from_notes(version: &str, body: &str) -> Self {
        let mut summary = String::new();
        let mut notes = Vec::new();
        for line in body.lines().map(str::trim) {
            if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                if notes.len() < NOTES_SHOWN {
                    notes.push(plain(item));
                }
            } else if summary.is_empty() && !line.is_empty() && !line.starts_with('#') {
                summary = plain(line.strip_prefix("Summary:").unwrap_or(line).trim());
            }
        }
        Self {
            version: version.into(),
            summary,
            notes,
        }
    }
}

/// Markdown emphasis, code and links reduced to their text.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '`' => {}
            '[' => {
                let label: String = chars.by_ref().take_while(|&c| c != ']').collect();
                if chars.peek() == Some(&'(') {
                    for c in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&plain(&label));
            }
            c => out.push(c),
        }
    }
    out.trim().to_owned()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Toast {
    Available { version: String },
    Updated { version: String, summary: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    Network(String),
    Signature,
    Install(String),
    Other(String),
}

impl Failure {
    pub fn message(&self, step: Step) -> String {
        match (self, step) {
            (Failure::Network(_), Step::Check) => {
                "DIO could not reach the update server. Check your internet connection.".into()
            }
            (Failure::Network(_), _) => "The connection was interrupted.".into(),
            (Failure::Signature, _) => "The update could not be verified. Its signature does not match DIO’s signing key, so it was discarded.".into(),
            (Failure::Install(error), _) => format!("The installer could not be started: {error}"),
            (Failure::Other(error), _) => error.clone(),
        }
    }
}

/// What the user is doing, sampled when a downloaded update is waiting to install.
#[derive(Clone, Copy, Debug, Default)]
pub struct Activity {
    pub popup_visible: bool,
    pub speaking: bool,
    /// Capturing a selection or downloading the voice model.
    pub busy: bool,
    pub idle_seconds: u64,
}

impl Activity {
    pub fn safe_to_restart(&self) -> bool {
        !self.popup_visible
            && !self.speaking
            && !self.busy
            && self.idle_seconds >= IDLE_BEFORE_INSTALL
    }
}

fn version(text: &str) -> Option<Version> {
    Version::parse(text.trim_start_matches('v')).ok()
}

pub struct Machine {
    pub state: UpdateState,
    pub record: Record,
    summary: String,
    received: u64,
    /// The status before a background check, restored if that check fails.
    before_check: Status,
}

impl Machine {
    pub fn new(current: &str, record: Record) -> Self {
        Self {
            state: UpdateState {
                status: Status::Idle,
                current: current.into(),
                version: String::new(),
                notes: Vec::new(),
                progress: 0.0,
                error: String::new(),
                retry: Step::None,
                last_checked: record.last_checked,
            },
            record,
            summary: String::new(),
            received: 0,
            before_check: Status::Idle,
        }
    }

    /// Settles the last installation: the installed version is running, or the installer failed.
    pub fn startup(&mut self) -> Option<Toast> {
        let pending = self.record.pending.take()?;
        let (running, wanted) = (version(&self.state.current)?, version(&pending.version)?);
        if running == wanted {
            self.state.status = Status::Updated;
            self.state.notes = Some(pending.summary.clone())
                .filter(|summary| !summary.is_empty())
                .into_iter()
                .collect();
            if self.record.notified_version.as_deref() != Some(&self.state.current) {
                self.record.notified_version = Some(self.state.current.clone());
                return Some(Toast::Updated {
                    version: self.state.current.clone(),
                    summary: pending.summary,
                });
            }
        } else if running < wanted {
            self.state.version = pending.version;
            self.fail(Step::Install, "The installer did not finish.".into());
        }
        None
    }

    fn busy(&self) -> bool {
        matches!(
            self.state.status,
            Status::Checking | Status::Downloading | Status::Verifying | Status::Installing
        )
    }

    fn downloaded_update(&self) -> bool {
        matches!(self.state.status, Status::Ready | Status::RestartRequired)
    }

    /// Starts a check unless one is running or an update is already waiting to install.
    pub fn begin_check(&mut self) -> bool {
        if self.busy() || self.downloaded_update() {
            return false;
        }
        self.before_check = self.state.status;
        self.state.status = Status::Checking;
        self.state.error.clear();
        self.state.retry = Step::None;
        true
    }

    pub fn checked(&mut self, now: i64, release: Option<Release>) {
        self.record.last_checked = Some(now);
        self.state.last_checked = Some(now);
        // The updater only offers newer versions; this guards against a downgrade regardless.
        let newer = release.filter(|release| {
            matches!((version(&release.version), version(&self.state.current)), (Some(remote), Some(current)) if remote > current)
        });
        match newer {
            Some(release) => {
                self.state.status = Status::Available;
                self.state.version = release.version;
                self.state.notes = release.notes;
                self.summary = release.summary;
            }
            None => {
                self.state.status = Status::UpToDate;
                self.state.version.clear();
                self.state.notes.clear();
                self.summary.clear();
            }
        }
    }

    /// A check the user didn't ask for fails quietly and is tried again at the next tick.
    pub fn check_failed(&mut self, background: bool, failure: &Failure) {
        if background {
            self.state.status = self.before_check;
        } else {
            self.fail(Step::Check, failure.message(Step::Check));
        }
    }

    pub fn begin_download(&mut self) -> bool {
        let retrying = self.state.status == Status::Failed && self.state.retry == Step::Download;
        if self.state.status != Status::Available && !retrying {
            return false;
        }
        self.state.status = Status::Downloading;
        self.state.progress = 0.0;
        self.state.error.clear();
        self.state.retry = Step::None;
        self.received = 0;
        true
    }

    /// Adds a downloaded chunk. Returns true when the whole percentage changed and is worth showing.
    pub fn progress(&mut self, chunk: u64, total: Option<u64>) -> bool {
        if self.state.status != Status::Downloading {
            return false;
        }
        self.received += chunk;
        let Some(total) = total.filter(|total| *total > 0) else {
            return false;
        };
        let next = (self.received as f64 / total as f64).min(1.0);
        let changed = (next * 100.0).floor() != (self.state.progress * 100.0).floor();
        self.state.progress = next;
        changed
    }

    pub fn verifying(&mut self) {
        if self.state.status == Status::Downloading {
            self.state.status = Status::Verifying;
            self.state.progress = 1.0;
        }
    }

    /// The update was downloaded and its signature verified. Returns false if it was cancelled meanwhile.
    pub fn downloaded(&mut self, background: bool) -> bool {
        if !matches!(self.state.status, Status::Downloading | Status::Verifying) {
            return false;
        }
        self.state.status = if background {
            Status::RestartRequired
        } else {
            Status::Ready
        };
        self.state.progress = 1.0;
        true
    }

    pub fn download_failed(&mut self, failure: &Failure) -> bool {
        if !matches!(self.state.status, Status::Downloading | Status::Verifying) {
            return false;
        }
        self.fail(Step::Download, failure.message(Step::Download));
        true
    }

    /// Cancelling is only offered while bytes are arriving; verification takes a moment and is
    /// left to finish.
    pub fn cancel(&mut self) -> bool {
        if self.state.status != Status::Downloading {
            return false;
        }
        self.state.status = Status::Available;
        self.state.progress = 0.0;
        self.received = 0;
        true
    }

    pub fn postpone(&mut self) -> bool {
        if self.state.status != Status::Ready {
            return false;
        }
        self.state.status = Status::RestartRequired;
        true
    }

    /// Marks the update as handed to the installer, so the next start can tell whether it worked.
    pub fn begin_install(&mut self) -> bool {
        if !self.downloaded_update() {
            return false;
        }
        self.state.status = Status::Installing;
        self.record.pending = Some(Pending {
            version: self.state.version.clone(),
            summary: self.summary.clone(),
        });
        true
    }

    pub fn fail(&mut self, step: Step, message: String) {
        self.state.status = Status::Failed;
        self.state.retry = step;
        self.state.error = message;
        self.state.progress = 0.0;
        if step == Step::Install {
            self.record.pending = None;
        }
    }

    pub fn dismiss(&mut self) -> bool {
        if !matches!(
            self.state.status,
            Status::Failed | Status::Updated | Status::UpToDate
        ) {
            return false;
        }
        self.state.status = Status::Idle;
        self.state.error.clear();
        self.state.retry = Step::None;
        true
    }

    /// Hides the offer and holds back its notification for a day.
    pub fn remind_later(&mut self, now: i64) -> bool {
        if self.state.status != Status::Available {
            return false;
        }
        self.state.status = Status::Idle;
        self.record.remind = Some(Remind {
            version: self.state.version.clone(),
            until: now + DAY,
        });
        true
    }

    /// Manual mode never checks on its own; the others check at most once a day.
    pub fn check_due(&self, now: i64, mode: &str) -> bool {
        mode != "manual"
            && !self.busy()
            && !self.downloaded_update()
            && self
                .record
                .last_checked
                .is_none_or(|last| now - last >= DAY || now < last)
    }

    /// Notify mode announces each version once, and again a day after "Later".
    pub fn should_announce(&self, now: i64) -> bool {
        if self.state.status != Status::Available {
            return false;
        }
        match &self.record.remind {
            Some(remind) if remind.version == self.state.version => now >= remind.until,
            _ => self.record.announced.as_deref() != Some(&self.state.version),
        }
    }

    pub fn announced(&mut self) {
        self.record.announced = Some(self.state.version.clone());
        self.record.remind = None;
    }
}

pub type Pinned<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type Progress<'a> = Box<dyn FnMut(u64, Option<u64>) + Send + 'a>;
pub type Finished<'a> = Box<dyn FnOnce() + Send + 'a>;

/// Where updates come from. `download` calls `finished` once every byte has arrived and before the
/// signature is checked, and only returns bytes whose signature is valid.
pub trait Source: Send + Sync {
    fn check(&self) -> Pinned<'_, Result<Option<Release>, Failure>>;
    fn download<'a>(
        &'a self,
        progress: Progress<'a>,
        finished: Finished<'a>,
    ) -> Pinned<'a, Result<Vec<u8>, Failure>>;
    /// Starts the installer. On success in production DIO exits, so this only returns on failure.
    fn install(&self, bytes: &[u8]) -> Result<(), Failure>;
}

pub trait Sink: Send + Sync {
    fn state(&self, state: &UpdateState);
    fn toast(&self, toast: Toast);
}

pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

pub struct Updates {
    machine: Mutex<Machine>,
    source: Arc<dyn Source>,
    sink: Arc<dyn Sink>,
    path: PathBuf,
    clock: Clock,
    task: Mutex<Option<JoinHandle<()>>>,
    /// The verified installer, kept only while an update waits to install.
    bytes: Mutex<Option<Vec<u8>>>,
}

impl Updates {
    pub fn new(
        current: &str,
        path: PathBuf,
        source: Arc<dyn Source>,
        sink: Arc<dyn Sink>,
        clock: Clock,
    ) -> Arc<Self> {
        let record = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let mut machine = Machine::new(current, record);
        let toast = machine.startup();
        let updates = Arc::new(Self {
            machine: Mutex::new(machine),
            source,
            sink,
            path,
            clock,
            task: Mutex::new(None),
            bytes: Mutex::new(None),
        });
        updates.save(&updates.machine.lock().unwrap());
        if let Some(toast) = toast {
            updates.sink.toast(toast);
        }
        updates
    }

    pub fn state(&self) -> UpdateState {
        self.machine.lock().unwrap().state.clone()
    }

    fn save(&self, machine: &Machine) {
        if let Err(error) = crate::storage::write_json(&self.path, &machine.record) {
            eprintln!("Could not save update record: {error}");
        }
    }

    fn publish(&self, machine: &Machine) {
        self.sink.state(&machine.state);
    }

    fn spawn(&self, work: impl Future<Output = ()> + Send + 'static) {
        *self.task.lock().unwrap() = Some(async_runtime::spawn(work));
    }

    /// Checks for an update. In auto mode a new version starts downloading in the background; in
    /// notify mode it is announced once.
    pub fn check(self: &Arc<Self>, mode: &str, background: bool) -> bool {
        {
            let mut machine = self.machine.lock().unwrap();
            if !machine.begin_check() {
                return false;
            }
            if !background {
                self.publish(&machine);
            }
        }
        let this = self.clone();
        let mode = mode.to_owned();
        self.spawn(async move {
            let result = this.source.check().await;
            let now = (this.clock)();
            let (download, toast) = {
                let mut machine = this.machine.lock().unwrap();
                match result {
                    Ok(release) => machine.checked(now, release),
                    Err(failure) => machine.check_failed(background, &failure),
                }
                let available = machine.state.status == Status::Available;
                let toast = (mode == "notify" && machine.should_announce(now)).then(|| {
                    machine.announced();
                    Toast::Available {
                        version: machine.state.version.clone(),
                    }
                });
                this.save(&machine);
                this.publish(&machine);
                (available && mode == "auto", toast)
            };
            if let Some(toast) = toast {
                this.sink.toast(toast);
            }
            if download {
                this.download(true);
            }
        });
        true
    }

    /// Downloads and verifies the available update. A background download waits for a safe moment
    /// to install; one the user started offers to restart now.
    pub fn download(self: &Arc<Self>, background: bool) -> bool {
        {
            let mut machine = self.machine.lock().unwrap();
            if !machine.begin_download() {
                return false;
            }
            self.publish(&machine);
        }
        let this = self.clone();
        self.spawn(async move {
            let progress = |chunk: u64, total: Option<u64>| {
                let mut machine = this.machine.lock().unwrap();
                if machine.progress(chunk, total) {
                    this.publish(&machine);
                }
            };
            let finished = || {
                let mut machine = this.machine.lock().unwrap();
                machine.verifying();
                this.publish(&machine);
            };
            let result = this
                .source
                .download(Box::new(progress), Box::new(finished))
                .await;
            let mut machine = this.machine.lock().unwrap();
            let applied = match result {
                Ok(bytes) => {
                    let applied = machine.downloaded(background);
                    if applied {
                        *this.bytes.lock().unwrap() = Some(bytes);
                    }
                    applied
                }
                Err(failure) => machine.download_failed(&failure),
            };
            if applied {
                this.publish(&machine);
            }
        });
        true
    }

    /// Stops a download in progress. Nothing is written until verification, so nothing is left behind.
    pub fn cancel(&self) {
        let mut machine = self.machine.lock().unwrap();
        if machine.cancel() {
            if let Some(task) = self.task.lock().unwrap().take() {
                task.abort();
            }
            *self.bytes.lock().unwrap() = None;
            self.publish(&machine);
        }
    }

    /// Hands the verified update to the installer. In production DIO exits here; this only returns
    /// if the installer could not be started, leaving the current version untouched.
    pub fn install(&self) {
        let Some(bytes) = self.bytes.lock().unwrap().take() else {
            let mut machine = self.machine.lock().unwrap();
            if machine.downloaded_update() {
                machine.fail(
                    Step::Download,
                    "The downloaded update is no longer available. Download it again.".into(),
                );
                self.publish(&machine);
            }
            return;
        };
        {
            let mut machine = self.machine.lock().unwrap();
            if !machine.begin_install() {
                *self.bytes.lock().unwrap() = Some(bytes);
                return;
            }
            self.save(&machine);
            self.publish(&machine);
        }
        if let Err(failure) = self.source.install(&bytes) {
            let mut machine = self.machine.lock().unwrap();
            machine.fail(Step::Install, failure.message(Step::Install));
            *self.bytes.lock().unwrap() = Some(bytes);
            self.save(&machine);
            self.publish(&machine);
        }
    }

    pub fn retry(self: &Arc<Self>, mode: &str) {
        let step = self.machine.lock().unwrap().state.retry;
        match step {
            Step::Download => {
                self.download(false);
            }
            Step::Install if self.bytes.lock().unwrap().is_some() => {
                let mut machine = self.machine.lock().unwrap();
                machine.state.status = Status::Ready;
                self.publish(&machine);
                drop(machine);
                self.install();
            }
            // After a failed install the downloaded copy is gone, so look for the update again.
            Step::Install | Step::Check | Step::None => {
                self.check(mode, false);
            }
        }
    }

    pub fn postpone(&self) {
        let mut machine = self.machine.lock().unwrap();
        if machine.postpone() {
            self.publish(&machine);
        }
    }

    pub fn dismiss(&self) {
        let mut machine = self.machine.lock().unwrap();
        if machine.dismiss() {
            self.publish(&machine);
        }
    }

    pub fn remind_later(&self) {
        let mut machine = self.machine.lock().unwrap();
        if machine.remind_later((self.clock)()) {
            self.save(&machine);
            self.publish(&machine);
        }
    }

    /// One scheduler tick: check when due, and install a waiting update once the user is away.
    pub fn tick(self: &Arc<Self>, mode: &str, activity: Activity) {
        let (due, waiting) = {
            let machine = self.machine.lock().unwrap();
            (
                machine.check_due((self.clock)(), mode),
                machine.state.status == Status::RestartRequired,
            )
        };
        if due {
            self.check(mode, true);
        } else if waiting && activity.safe_to_restart() {
            self.install();
        }
    }

    #[cfg(test)]
    async fn settle(&self) {
        let task = self.task.lock().unwrap().take();
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}

/// Runs the scheduler for as long as DIO runs. `mode` and `activity` are read at every tick, so
/// preference changes apply without a restart.
pub fn schedule(
    updates: Arc<Updates>,
    mode: impl Fn() -> String + Send + 'static,
    activity: impl Fn() -> Activity + Send + 'static,
) {
    async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            updates.tick(&mode(), activity());
            tokio::time::sleep(TICK).await;
        }
    });
}

/// The Tauri updater, configured from `plugins.updater` in `tauri.conf.json`.
pub struct PluginSource {
    app: tauri::AppHandle,
    update: Mutex<Option<tauri_plugin_updater::Update>>,
}

impl PluginSource {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            update: Mutex::new(None),
        }
    }
}

fn classify(error: tauri_plugin_updater::Error) -> Failure {
    use tauri_plugin_updater::Error;
    match error {
        Error::Reqwest(_) | Error::Network(_) | Error::ReleaseNotFound => {
            Failure::Network(error.to_string())
        }
        Error::Minisign(_)
        | Error::Base64(_)
        | Error::SignatureUtf8(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion => Failure::Signature,
        error => Failure::Other(error.to_string()),
    }
}

impl Source for PluginSource {
    fn check(&self) -> Pinned<'_, Result<Option<Release>, Failure>> {
        Box::pin(async move {
            use tauri_plugin_updater::UpdaterExt;
            let updater = self
                .app
                .updater_builder()
                .version_comparator(|current, remote| remote.version > current)
                // No overall timeout, which would cut off slow downloads; a stalled connection
                // still fails after a minute without data.
                .configure_client(|client| {
                    client
                        .connect_timeout(Duration::from_secs(15))
                        .read_timeout(Duration::from_secs(60))
                })
                .build()
                .map_err(classify)?;
            let update = updater.check().await.map_err(classify)?;
            let release = update.as_ref().map(|update| {
                Release::from_notes(&update.version, update.body.as_deref().unwrap_or_default())
            });
            *self.update.lock().unwrap() = update;
            Ok(release)
        })
    }

    fn download<'a>(
        &'a self,
        mut progress: Progress<'a>,
        finished: Finished<'a>,
    ) -> Pinned<'a, Result<Vec<u8>, Failure>> {
        Box::pin(async move {
            let update = self.update.lock().unwrap().clone().ok_or_else(|| {
                Failure::Other("Check for updates again before downloading.".into())
            })?;
            update
                .download(|chunk, total| progress(chunk as u64, total), finished)
                .await
                .map_err(classify)
        })
    }

    fn install(&self, bytes: &[u8]) -> Result<(), Failure> {
        let update = self
            .update
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| Failure::Install("the update is no longer available".into()))?;
        update
            .install(bytes)
            .map_err(|error| match classify(error) {
                Failure::Signature => Failure::Signature,
                failure => Failure::Install(match failure {
                    Failure::Network(error) | Failure::Other(error) | Failure::Install(error) => {
                        error
                    }
                    Failure::Signature => unreachable!(),
                }),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    fn release(version: &str) -> Release {
        Release {
            version: version.into(),
            summary: "Faster lookups.".into(),
            notes: vec!["Faster lookups.".into()],
        }
    }

    /// Plays back scripted results, one per call.
    #[derive(Default)]
    struct Fake {
        checks: Mutex<VecDeque<Result<Option<Release>, Failure>>>,
        downloads: Mutex<VecDeque<Result<Vec<u8>, Failure>>>,
        installs: Mutex<VecDeque<Result<(), Failure>>>,
        /// When set, downloads report one chunk and then wait until released.
        hold: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
        check_calls: AtomicUsize,
        installed: Mutex<Vec<Vec<u8>>>,
    }
    impl Source for Fake {
        fn check(&self) -> Pinned<'_, Result<Option<Release>, Failure>> {
            self.check_calls.fetch_add(1, Ordering::SeqCst);
            let result = self.checks.lock().unwrap().pop_front().unwrap_or(Ok(None));
            Box::pin(async move { result })
        }
        fn download<'a>(
            &'a self,
            mut progress: Progress<'a>,
            finished: Finished<'a>,
        ) -> Pinned<'a, Result<Vec<u8>, Failure>> {
            let result = self
                .downloads
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(vec![1]));
            let hold = self.hold.lock().unwrap().take();
            Box::pin(async move {
                progress(50, Some(100));
                if let Some(hold) = hold {
                    let _ = hold.await;
                }
                progress(50, Some(100));
                if !matches!(result, Err(Failure::Network(_))) {
                    finished();
                }
                result
            })
        }
        fn install(&self, bytes: &[u8]) -> Result<(), Failure> {
            self.installed.lock().unwrap().push(bytes.to_vec());
            self.installs.lock().unwrap().pop_front().unwrap_or(Ok(()))
        }
    }

    #[derive(Default)]
    struct Log {
        states: Mutex<Vec<Status>>,
        toasts: Mutex<Vec<Toast>>,
    }
    impl Sink for Log {
        fn state(&self, state: &UpdateState) {
            self.states.lock().unwrap().push(state.status);
        }
        fn toast(&self, toast: Toast) {
            self.toasts.lock().unwrap().push(toast);
        }
    }

    struct Harness {
        dir: PathBuf,
        source: Arc<Fake>,
        log: Arc<Log>,
        now: Arc<AtomicI64>,
    }
    impl Harness {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("dio-updates-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self {
                dir,
                source: Arc::new(Fake::default()),
                log: Arc::new(Log::default()),
                now: Arc::new(AtomicI64::new(1_000_000)),
            }
        }
        fn now(&self) -> i64 {
            self.now.load(Ordering::SeqCst)
        }
        fn advance(&self, seconds: i64) {
            self.now.fetch_add(seconds, Ordering::SeqCst);
        }
        /// Starts DIO at `version`, reading the record a previous run left behind.
        fn start(&mut self, version: &str) -> Arc<Updates> {
            self.log = Arc::new(Log::default());
            Updates::new(
                version,
                self.dir.join("update.json"),
                self.source.clone(),
                self.log.clone(),
                {
                    let now = self.now.clone();
                    Arc::new(move || now.load(Ordering::SeqCst))
                },
            )
        }
        fn record(&self) -> Record {
            serde_json::from_slice(&std::fs::read(self.dir.join("update.json")).unwrap()).unwrap()
        }
        fn toasts(&self) -> Vec<Toast> {
            self.log.toasts.lock().unwrap().clone()
        }
    }
    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
    fn run<F: Future>(future: F) -> F::Output {
        async_runtime::block_on(future)
    }

    #[test]
    fn release_notes_are_reduced_to_summary_and_plain_bullets() {
        let notes = "Summary: Faster **offline** lookups.\n\n### New features\n\n- **Updates:** DIO installs [new versions](https://example.com) itself.\n- Uses `update.json` for state.\n* Snake_case and _emphasis_ stay readable.\n";
        let release = Release::from_notes("0.2.1", notes);
        assert_eq!(release.summary, "Faster offline lookups.");
        assert_eq!(
            release.notes,
            [
                "Updates: DIO installs new versions itself.",
                "Uses update.json for state.",
                "Snake_case and _emphasis_ stay readable."
            ]
        );
        let many = (0..9).map(|n| format!("- item {n}\n")).collect::<String>();
        assert_eq!(Release::from_notes("1.0.0", &many).notes.len(), NOTES_SHOWN);
    }

    #[test]
    fn check_finds_an_update_or_reports_up_to_date() {
        let mut machine = Machine::new("0.2.0", Record::default());
        assert!(machine.begin_check());
        assert!(
            !machine.begin_check(),
            "a second check while one runs is refused"
        );
        machine.checked(10, Some(release("0.2.1")));
        assert_eq!(machine.state.status, Status::Available);
        assert_eq!(machine.state.last_checked, Some(10));
        assert!(machine.begin_check());
        machine.checked(20, None);
        assert_eq!(machine.state.status, Status::UpToDate);
        assert!(machine.state.version.is_empty());
    }

    #[test]
    fn older_or_equal_versions_are_never_offered() {
        for offered in ["0.1.9", "0.2.0", "not-a-version"] {
            let mut machine = Machine::new("0.2.0", Record::default());
            machine.begin_check();
            machine.checked(1, Some(release(offered)));
            assert_eq!(machine.state.status, Status::UpToDate, "{offered}");
        }
    }

    #[test]
    fn manual_mode_never_schedules_and_others_check_daily() {
        let machine = Machine::new("0.2.0", Record::default());
        assert!(!machine.check_due(0, "manual"));
        assert!(machine.check_due(0, "auto"));
        assert!(machine.check_due(0, "notify"));
        let checked = Machine::new(
            "0.2.0",
            Record {
                last_checked: Some(1000),
                ..Record::default()
            },
        );
        assert!(!checked.check_due(1000 + DAY - 1, "auto"));
        assert!(checked.check_due(1000 + DAY, "auto"));
        assert!(
            checked.check_due(500, "auto"),
            "a clock set back does not block checks forever"
        );
    }

    #[test]
    fn last_check_survives_a_restart_so_restarts_add_no_requests() {
        let mut harness = Harness::new("throttle");
        harness.source.checks.lock().unwrap().push_back(Ok(None));
        let updates = harness.start("0.2.0");
        updates.tick("auto", Activity::default());
        run(updates.settle());
        assert_eq!(harness.source.check_calls.load(Ordering::SeqCst), 1);
        let restarted = harness.start("0.2.0");
        restarted.tick("auto", Activity::default());
        run(restarted.settle());
        assert_eq!(harness.source.check_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.record().last_checked, Some(harness.now()));
    }

    #[test]
    fn manual_mode_tick_makes_no_requests() {
        let mut harness = Harness::new("manual");
        let updates = harness.start("0.2.0");
        for _ in 0..3 {
            updates.tick("manual", Activity::default());
        }
        run(updates.settle());
        assert_eq!(harness.source.check_calls.load(Ordering::SeqCst), 0);
        updates.check("manual", false);
        run(updates.settle());
        assert_eq!(harness.source.check_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn download_reports_progress_then_verification_then_ready() {
        let mut harness = Harness::new("download");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        let updates = harness.start("0.2.0");
        updates.check("notify", false);
        run(updates.settle());
        assert_eq!(updates.state().status, Status::Available);
        updates.download(false);
        run(updates.settle());
        assert_eq!(updates.state().status, Status::Ready);
        let states = harness.log.states.lock().unwrap().clone();
        let tail: Vec<Status> = states
            .into_iter()
            .skip_while(|s| *s != Status::Downloading)
            .collect();
        assert_eq!(
            tail,
            [
                Status::Downloading,
                Status::Downloading,
                Status::Downloading,
                Status::Verifying,
                Status::Ready
            ]
        );
    }

    #[test]
    fn auto_mode_downloads_in_the_background_and_waits_for_the_user_to_be_away() {
        let mut harness = Harness::new("auto");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        let updates = harness.start("0.2.0");
        updates.tick("auto", Activity::default());
        run(updates.settle()); // check, which starts the download
        run(updates.settle()); // download
        assert_eq!(updates.state().status, Status::RestartRequired);
        assert!(harness.toasts().is_empty(), "auto mode does not announce");
        let using = Activity {
            idle_seconds: IDLE_BEFORE_INSTALL,
            popup_visible: true,
            ..Activity::default()
        };
        updates.tick("auto", using);
        updates.tick(
            "auto",
            Activity {
                idle_seconds: 30,
                ..Activity::default()
            },
        );
        updates.tick(
            "auto",
            Activity {
                idle_seconds: IDLE_BEFORE_INSTALL,
                speaking: true,
                ..Activity::default()
            },
        );
        assert!(harness.source.installed.lock().unwrap().is_empty());
        updates.tick(
            "auto",
            Activity {
                idle_seconds: IDLE_BEFORE_INSTALL,
                ..Activity::default()
            },
        );
        assert_eq!(harness.source.installed.lock().unwrap().len(), 1);
        assert_eq!(updates.state().status, Status::Installing);
        assert_eq!(
            harness.record().pending.map(|p| p.version).as_deref(),
            Some("0.2.1")
        );
    }

    #[test]
    fn cancelling_a_download_discards_it_and_keeps_the_offer() {
        let mut harness = Harness::new("cancel");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        let (release_download, hold) = tokio::sync::oneshot::channel();
        *harness.source.hold.lock().unwrap() = Some(hold);
        let updates = harness.start("0.2.0");
        updates.check("notify", false);
        run(updates.settle());
        updates.download(false);
        run(async {
            while updates.state().progress < 0.5 {
                tokio::task::yield_now().await;
            }
        });
        updates.cancel();
        let _ = release_download.send(());
        run(updates.settle());
        let state = updates.state();
        assert_eq!((state.status, state.progress), (Status::Available, 0.0));
        assert!(updates.bytes.lock().unwrap().is_none());
        updates.install();
        assert!(harness.source.installed.lock().unwrap().is_empty());
    }

    #[test]
    fn network_failures_are_reported_and_retried() {
        let mut harness = Harness::new("network");
        fn network<T>() -> Result<T, Failure> {
            Err(Failure::Network("connection reset".into()))
        }
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .extend([network(), Ok(Some(release("0.2.1")))]);
        harness
            .source
            .downloads
            .lock()
            .unwrap()
            .extend([network(), Ok(vec![7])]);
        let updates = harness.start("0.2.0");
        updates.check("notify", false);
        run(updates.settle());
        let state = updates.state();
        assert_eq!((state.status, state.retry), (Status::Failed, Step::Check));
        assert!(state.error.contains("internet connection"));
        updates.retry("notify");
        run(updates.settle());
        assert_eq!(updates.state().status, Status::Available);
        updates.download(false);
        run(updates.settle());
        let state = updates.state();
        assert_eq!(
            (state.status, state.retry, state.error.as_str()),
            (
                Status::Failed,
                Step::Download,
                "The connection was interrupted."
            )
        );
        updates.retry("notify");
        run(updates.settle());
        assert_eq!(updates.state().status, Status::Ready);
    }

    #[test]
    fn background_check_failures_stay_quiet() {
        let mut harness = Harness::new("quiet");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Err(Failure::Network("offline".into())));
        let updates = harness.start("0.2.0");
        updates.tick("notify", Activity::default());
        run(updates.settle());
        assert_eq!(updates.state().status, Status::Idle);
        assert_eq!(
            updates.state().last_checked,
            None,
            "a failed check is retried at the next tick"
        );
    }

    #[test]
    fn invalid_signature_discards_the_download() {
        let mut harness = Harness::new("signature");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        harness
            .source
            .downloads
            .lock()
            .unwrap()
            .push_back(Err(Failure::Signature));
        let updates = harness.start("0.2.0");
        updates.check("notify", false);
        run(updates.settle());
        updates.download(false);
        run(updates.settle());
        let state = updates.state();
        assert_eq!(
            (state.status, state.retry),
            (Status::Failed, Step::Download)
        );
        assert!(state.error.contains("could not be verified"));
        assert!(updates.bytes.lock().unwrap().is_none());
        updates.install();
        assert!(
            harness.source.installed.lock().unwrap().is_empty(),
            "nothing unverified is installed"
        );
    }

    #[test]
    fn installer_that_fails_to_start_keeps_the_download_for_a_retry() {
        let mut harness = Harness::new("install");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        harness
            .source
            .installs
            .lock()
            .unwrap()
            .push_back(Err(Failure::Install("access denied".into())));
        let updates = harness.start("0.2.0");
        updates.check("manual", false);
        run(updates.settle());
        updates.download(false);
        run(updates.settle());
        updates.install();
        let state = updates.state();
        assert_eq!((state.status, state.retry), (Status::Failed, Step::Install));
        assert!(
            harness.record().pending.is_none(),
            "no install is expected at the next start"
        );
        updates.retry("manual");
        assert_eq!(updates.state().status, Status::Installing);
        assert_eq!(harness.source.installed.lock().unwrap().len(), 2);
    }

    #[test]
    fn successful_update_is_announced_once_after_restart() {
        let mut harness = Harness::new("success");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        let updates = harness.start("0.2.0");
        updates.check("notify", false);
        run(updates.settle());
        updates.download(false);
        run(updates.settle());
        updates.install();
        drop(updates);
        // The installer replaced DIO and started the new version.
        let updated = harness.start("0.2.1");
        assert_eq!(updated.state().status, Status::Updated);
        assert_eq!(
            harness.toasts(),
            [Toast::Updated {
                version: "0.2.1".into(),
                summary: "Faster lookups.".into()
            }]
        );
        assert!(harness.record().pending.is_none());
        let again = harness.start("0.2.1");
        assert_eq!(again.state().status, Status::Idle);
        assert!(
            harness.toasts().is_empty(),
            "the success notification is shown once"
        );
    }

    #[test]
    fn interrupted_install_is_detected_at_the_next_start() {
        let mut harness = Harness::new("interrupted");
        std::fs::write(
            harness.dir.join("update.json"),
            r#"{"pending":{"version":"0.2.1","summary":"Faster lookups."}}"#,
        )
        .unwrap();
        // The installer never finished, so the old version started again.
        let updates = harness.start("0.2.0");
        let state = updates.state();
        assert_eq!(
            (state.status, state.retry, state.version.as_str()),
            (Status::Failed, Step::Install, "0.2.1")
        );
        assert!(harness.toasts().is_empty());
        assert!(harness.record().pending.is_none());
        // Retrying looks for the update again, since the downloaded copy is gone.
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        updates.retry("manual");
        run(updates.settle());
        assert_eq!(updates.state().status, Status::Available);
    }

    #[test]
    fn notify_mode_announces_each_version_once_and_again_after_later() {
        let mut harness = Harness::new("notify");
        harness.source.checks.lock().unwrap().extend([
            Ok(Some(release("0.2.1"))),
            Ok(Some(release("0.2.1"))),
            Ok(Some(release("0.2.1"))),
            Ok(Some(release("0.2.2"))),
        ]);
        let updates = harness.start("0.2.0");
        updates.tick("notify", Activity::default());
        run(updates.settle());
        assert_eq!(
            harness.toasts(),
            [Toast::Available {
                version: "0.2.1".into()
            }]
        );
        updates.remind_later();
        assert_eq!(updates.state().status, Status::Idle);
        harness.advance(DAY / 2);
        updates.check("notify", false);
        run(updates.settle());
        assert_eq!(harness.toasts().len(), 1, "held back during the snooze");
        harness.advance(DAY);
        updates.check("notify", false);
        run(updates.settle());
        assert_eq!(
            harness.toasts().len(),
            2,
            "announced again once the snooze ends"
        );
        updates.check("notify", false);
        run(updates.settle());
        assert_eq!(
            harness.toasts().last(),
            Some(&Toast::Available {
                version: "0.2.2".into()
            })
        );
        assert_eq!(harness.toasts().len(), 3);
    }

    #[test]
    fn duplicate_operations_are_ignored() {
        let mut machine = Machine::new("0.2.0", Record::default());
        assert!(!machine.begin_download(), "nothing to download yet");
        machine.begin_check();
        machine.checked(1, Some(release("0.2.1")));
        assert!(machine.begin_download());
        assert!(!machine.begin_download());
        assert!(!machine.begin_check(), "no check during a download");
        machine.verifying();
        assert!(!machine.cancel(), "verification is left to finish");
        assert!(machine.downloaded(false));
        assert!(
            !machine.begin_check(),
            "no check while an update waits to install"
        );
        assert!(machine.begin_install());
        assert!(!machine.begin_install());
    }

    #[test]
    fn later_on_a_ready_update_waits_for_a_safe_moment() {
        let mut harness = Harness::new("later");
        harness
            .source
            .checks
            .lock()
            .unwrap()
            .push_back(Ok(Some(release("0.2.1"))));
        let updates = harness.start("0.2.0");
        updates.check("notify", false);
        run(updates.settle());
        updates.download(false);
        run(updates.settle());
        updates.postpone();
        assert_eq!(updates.state().status, Status::RestartRequired);
        harness.advance(DAY);
        // Manual mode never checks, so the tick goes straight to the waiting update.
        updates.tick(
            "manual",
            Activity {
                idle_seconds: 60,
                ..Activity::default()
            },
        );
        assert!(harness.source.installed.lock().unwrap().is_empty());
        updates.tick(
            "manual",
            Activity {
                idle_seconds: IDLE_BEFORE_INSTALL,
                ..Activity::default()
            },
        );
        assert_eq!(harness.source.installed.lock().unwrap().len(), 1);
    }

    #[test]
    fn safe_moment_needs_an_idle_user_and_a_quiet_dio() {
        let away = Activity {
            idle_seconds: IDLE_BEFORE_INSTALL,
            ..Activity::default()
        };
        assert!(away.safe_to_restart());
        assert!(!Activity { busy: true, ..away }.safe_to_restart());
        assert!(!Activity {
            speaking: true,
            ..away
        }
        .safe_to_restart());
        assert!(!Activity {
            popup_visible: true,
            ..away
        }
        .safe_to_restart());
        assert!(!Activity {
            idle_seconds: IDLE_BEFORE_INSTALL - 1,
            ..away
        }
        .safe_to_restart());
    }

    #[test]
    fn failure_messages_match_the_settings_copy() {
        assert_eq!(
            Failure::Network("x".into()).message(Step::Download),
            "The connection was interrupted."
        );
        assert!(Failure::Network("x".into())
            .message(Step::Check)
            .contains("internet connection"));
        assert!(Failure::Signature
            .message(Step::Download)
            .contains("signature"));
        assert!(Failure::Install("denied".into())
            .message(Step::Install)
            .ends_with("denied"));
    }
}
