use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use vpm_core::{Clock, ProfileStore, VolumeProfile, match_profile};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IMMDevice, IMMDeviceCollection,
    IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
    eMultimedia, eRender,
};
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantToStringAlloc};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::core::{PCWSTR, implement};

pub const PKEY_DEVICE_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

pub const DEFAULT_DEBOUNCE_WINDOW: Duration = Duration::from_millis(800);
pub const DEFAULT_SAME_DEVICE_SUPPRESS_WINDOW: Duration = Duration::from_secs(3);

pub type ProfileAppliedCallback = Box<dyn Fn(ProfileAppliedEvent) + Send + Sync>;
type CallbackSlot = Arc<Mutex<Option<ProfileAppliedCallback>>>;

#[derive(Debug)]
pub enum AudioError {
    Com(windows::core::Error),
    DeviceNotFound(String),
    NoDefaultDevice,
    InvalidVolume(f32),
    Io(std::io::Error),
    Other(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Com(e) => write!(f, "COM error: {e}"),
            Self::DeviceNotFound(id) => write!(f, "Device not found: {id}"),
            Self::NoDefaultDevice => write!(f, "No default playback device found"),
            Self::InvalidVolume(v) => write!(f, "Invalid volume: {v}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Other(msg) => write!(f, "Audio error: {msg}"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<windows::core::Error> for AudioError {
    fn from(e: windows::core::Error) -> Self {
        Self::Com(e)
    }
}

impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceChangeType {
    DeviceAdded,
    DeviceRemoved,
    DeviceStateChanged,
    DefaultDeviceChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceChangedEventArgs {
    pub previous_device_id: String,
    pub new_device_id: String,
    pub change_type: DeviceChangeType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileAppliedEvent {
    pub device_id: String,
    pub device_name: String,
    pub master_volume: f32,
    pub is_muted: bool,
    pub is_new_profile: bool,
}

pub trait AudioDeviceAdapter: Send + Sync {
    fn get_default_playback_device(&self) -> Result<Option<AudioDeviceInfo>, AudioError>;
    fn get_playback_devices(&self) -> Result<Vec<AudioDeviceInfo>, AudioError>;
    fn get_master_volume(&self, device_id: Option<&str>) -> Result<f32, AudioError>;
    fn set_master_volume(&self, device_id: Option<&str>, volume: f32) -> Result<(), AudioError>;
    fn get_mute(&self, device_id: Option<&str>) -> Result<bool, AudioError>;
    fn set_mute(&self, device_id: Option<&str>, mute: bool) -> Result<(), AudioError>;
}

pub struct WindowsAudioDeviceAdapter;

impl WindowsAudioDeviceAdapter {
    pub fn new() -> Self {
        Self
    }

    fn init_com() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    fn create_enumerator() -> Result<IMMDeviceEnumerator, AudioError> {
        Self::init_com();
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(AudioError::Com) }
    }

    fn extract_device_info(
        device: &IMMDevice,
        is_default: bool,
    ) -> Result<AudioDeviceInfo, AudioError> {
        let device_id = unsafe {
            let id_pwstr = device.GetId()?;
            let id_str = id_pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(id_pwstr.as_ptr() as *const _));
            id_str
        };

        let device_name = unsafe {
            let store: IPropertyStore = device.OpenPropertyStore(STGM_READ)?;
            let prop: PROPVARIANT = store.GetValue(&PKEY_DEVICE_FRIENDLY_NAME)?;
            if let Ok(name_pwstr) = PropVariantToStringAlloc(&prop) {
                let name_str = name_pwstr.to_string().unwrap_or_default();
                CoTaskMemFree(Some(name_pwstr.as_ptr() as *const _));
                name_str
            } else {
                "(unknown)".to_string()
            }
        };

        Ok(AudioDeviceInfo {
            device_id,
            device_name,
            is_default,
        })
    }

    fn get_target_endpoint(&self, device_id: Option<&str>) -> Result<IMMDevice, AudioError> {
        let enumerator = Self::create_enumerator()?;
        unsafe {
            if let Some(id) = device_id {
                let wide_id: Vec<u16> = OsStr::new(id)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                enumerator
                    .GetDevice(PCWSTR(wide_id.as_ptr()))
                    .map_err(|_| AudioError::DeviceNotFound(id.to_string()))
            } else {
                enumerator
                    .GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .map_err(|_| AudioError::NoDefaultDevice)
            }
        }
    }

    fn get_volume_endpoint(
        &self,
        device_id: Option<&str>,
    ) -> Result<IAudioEndpointVolume, AudioError> {
        let device = self.get_target_endpoint(device_id)?;
        unsafe {
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(AudioError::Com)
        }
    }
}

impl Default for WindowsAudioDeviceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDeviceAdapter for WindowsAudioDeviceAdapter {
    fn get_default_playback_device(&self) -> Result<Option<AudioDeviceInfo>, AudioError> {
        let enumerator = Self::create_enumerator()?;
        let device = match unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) } {
            Ok(dev) => dev,
            Err(_) => return Ok(None),
        };

        let info = Self::extract_device_info(&device, true)?;
        Ok(Some(info))
    }

    fn get_playback_devices(&self) -> Result<Vec<AudioDeviceInfo>, AudioError> {
        let enumerator = Self::create_enumerator()?;
        let default_id = self
            .get_default_playback_device()
            .ok()
            .flatten()
            .map(|d| d.device_id);

        let mut devices = Vec::new();
        unsafe {
            let collection: IMMDeviceCollection = enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(AudioError::Com)?;
            let count = collection.GetCount().map_err(AudioError::Com)?;
            for i in 0..count {
                if let Ok(dev) = collection.Item(i)
                    && let Ok(info) = Self::extract_device_info(&dev, false)
                {
                    let is_default = default_id.as_deref() == Some(&info.device_id);
                    devices.push(AudioDeviceInfo { is_default, ..info });
                }
            }
        }

        Ok(devices)
    }

    fn get_master_volume(&self, device_id: Option<&str>) -> Result<f32, AudioError> {
        let volume_ep = self.get_volume_endpoint(device_id)?;
        unsafe {
            volume_ep
                .GetMasterVolumeLevelScalar()
                .map_err(AudioError::Com)
        }
    }

    fn set_master_volume(&self, device_id: Option<&str>, volume: f32) -> Result<(), AudioError> {
        if !volume.is_finite() {
            return Err(AudioError::InvalidVolume(volume));
        }
        let clamped = volume.clamp(0.0, 1.0);
        let volume_ep = self.get_volume_endpoint(device_id)?;
        unsafe {
            volume_ep
                .SetMasterVolumeLevelScalar(clamped, std::ptr::null())
                .map_err(AudioError::Com)
        }
    }

    fn get_mute(&self, device_id: Option<&str>) -> Result<bool, AudioError> {
        let volume_ep = self.get_volume_endpoint(device_id)?;
        let mute = unsafe { volume_ep.GetMute().map_err(AudioError::Com)? };
        Ok(mute.as_bool())
    }

    fn set_mute(&self, device_id: Option<&str>, mute: bool) -> Result<(), AudioError> {
        let volume_ep = self.get_volume_endpoint(device_id)?;
        unsafe {
            volume_ep
                .SetMute(mute, std::ptr::null())
                .map_err(AudioError::Com)
        }
    }
}

#[implement(IMMNotificationClient)]
pub struct ComNotificationClient {
    sender: Sender<DeviceChangedEventArgs>,
}

impl ComNotificationClient {
    pub fn new(sender: Sender<DeviceChangedEventArgs>) -> Self {
        Self { sender }
    }
}

impl IMMNotificationClient_Impl for ComNotificationClient_Impl {
    fn OnDeviceStateChanged(
        &self,
        pwstrdeviceid: &PCWSTR,
        _dwnewstate: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        let device_id = unsafe { pwstrdeviceid.to_string().unwrap_or_default() };
        let _ = self.sender.send(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: device_id,
            change_type: DeviceChangeType::DeviceStateChanged,
        });
        Ok(())
    }

    fn OnDeviceAdded(&self, pwstrdeviceid: &PCWSTR) -> windows::core::Result<()> {
        let device_id = unsafe { pwstrdeviceid.to_string().unwrap_or_default() };
        let _ = self.sender.send(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: device_id,
            change_type: DeviceChangeType::DeviceAdded,
        });
        Ok(())
    }

    fn OnDeviceRemoved(&self, pwstrdeviceid: &PCWSTR) -> windows::core::Result<()> {
        let device_id = unsafe { pwstrdeviceid.to_string().unwrap_or_default() };
        let _ = self.sender.send(DeviceChangedEventArgs {
            previous_device_id: device_id,
            new_device_id: String::new(),
            change_type: DeviceChangeType::DeviceRemoved,
        });
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: EDataFlow,
        role: ERole,
        pwstrdefaultdeviceid: &PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == eRender && role == eMultimedia {
            let device_id = unsafe { pwstrdefaultdeviceid.to_string().unwrap_or_default() };
            let _ = self.sender.send(DeviceChangedEventArgs {
                previous_device_id: String::new(),
                new_device_id: device_id,
                change_type: DeviceChangeType::DefaultDeviceChanged,
            });
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _pwstrdeviceid: &PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

pub struct DeviceMonitorService {
    enumerator: Option<IMMDeviceEnumerator>,
    client: Option<IMMNotificationClient>,
}

unsafe impl Send for DeviceMonitorService {}
unsafe impl Sync for DeviceMonitorService {}

impl DeviceMonitorService {
    pub fn start(sender: Sender<DeviceChangedEventArgs>) -> Result<Self, AudioError> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(AudioError::Com)?;
            let client: IMMNotificationClient = ComNotificationClient::new(sender).into();
            enumerator
                .RegisterEndpointNotificationCallback(&client)
                .map_err(AudioError::Com)?;
            Ok(Self {
                enumerator: Some(enumerator),
                client: Some(client),
            })
        }
    }
}

impl Drop for DeviceMonitorService {
    fn drop(&mut self) {
        if let (Some(enumerator), Some(client)) = (self.enumerator.take(), self.client.take()) {
            unsafe {
                let _ = enumerator.UnregisterEndpointNotificationCallback(&client);
            }
        }
    }
}

pub struct ProfileCaptureService<A: AudioDeviceAdapter, S: ProfileStore, C: Clock> {
    adapter: Arc<A>,
    store: Arc<S>,
    clock: Arc<C>,
}

impl<A: AudioDeviceAdapter, S: ProfileStore, C: Clock> ProfileCaptureService<A, S, C> {
    pub fn new(adapter: Arc<A>, store: Arc<S>, clock: Arc<C>) -> Self {
        Self {
            adapter,
            store,
            clock,
        }
    }

    pub fn capture_current_profile(
        &self,
        device_id: Option<&str>,
    ) -> Result<Option<VolumeProfile>, AudioError> {
        let device = if let Some(id) = device_id {
            let devices = self.adapter.get_playback_devices()?;
            devices.into_iter().find(|d| d.device_id == id)
        } else {
            self.adapter.get_default_playback_device()?
        };

        let device = match device {
            Some(d) => d,
            None => return Ok(None),
        };

        let vol = self.adapter.get_master_volume(Some(&device.device_id))?;
        let mute = self.adapter.get_mute(Some(&device.device_id))?;
        let now = self.clock.now();

        let profile = VolumeProfile {
            device_id: device.device_id.clone(),
            device_name: if device.device_name == "(unknown)" {
                String::new()
            } else {
                device.device_name
            },
            master_volume: vol,
            is_muted: mute,
            created_at: now,
            last_applied: now,
        };

        self.store.save(&profile).map_err(AudioError::Io)?;
        Ok(Some(profile))
    }
}

#[derive(Clone, Debug)]
struct AppliedState {
    device_id: String,
    applied_time: Duration,
}

pub struct OrchestratorEngine<A: AudioDeviceAdapter, S: ProfileStore, C: Clock> {
    adapter: Arc<A>,
    store: Arc<S>,
    clock: Arc<C>,
    debounce_window: Duration,
    same_device_suppress_window: Duration,
    last_applied: Option<AppliedState>,
    pending_settle_due: Option<Duration>,
    applied_events: Vec<ProfileAppliedEvent>,
    stopped: bool,
}

impl<A: AudioDeviceAdapter, S: ProfileStore, C: Clock> OrchestratorEngine<A, S, C> {
    pub fn new(
        adapter: Arc<A>,
        store: Arc<S>,
        clock: Arc<C>,
        debounce_window: Option<Duration>,
        same_device_suppress_window: Option<Duration>,
    ) -> Self {
        Self {
            adapter,
            store,
            clock,
            debounce_window: debounce_window.unwrap_or(DEFAULT_DEBOUNCE_WINDOW),
            same_device_suppress_window: same_device_suppress_window
                .unwrap_or(DEFAULT_SAME_DEVICE_SUPPRESS_WINDOW),
            last_applied: None,
            pending_settle_due: None,
            applied_events: Vec::new(),
            stopped: false,
        }
    }

    pub fn on_device_changed(
        &mut self,
        event: DeviceChangedEventArgs,
    ) -> Result<Option<ProfileAppliedEvent>, AudioError> {
        if self.stopped || event.new_device_id.is_empty() {
            return Ok(None);
        }

        let now_elapsed = self.clock.elapsed();
        self.pending_settle_due = Some(now_elapsed + self.debounce_window);

        if event.change_type == DeviceChangeType::DefaultDeviceChanged {
            self.resolve_and_apply()
        } else {
            Ok(None)
        }
    }

    pub fn advance_time(&mut self) -> Result<Vec<ProfileAppliedEvent>, AudioError> {
        if self.stopped {
            return Ok(Vec::new());
        }

        let current_elapsed = self.clock.elapsed();
        let mut triggered = Vec::new();

        if let Some(due) = self.pending_settle_due
            && current_elapsed >= due
        {
            self.pending_settle_due = None;
            if let Some(event) = self.resolve_and_apply()? {
                triggered.push(event);
            }
        }

        Ok(triggered)
    }

    pub fn resolve_and_apply(&mut self) -> Result<Option<ProfileAppliedEvent>, AudioError> {
        if self.stopped {
            return Ok(None);
        }

        let default_device = match self.adapter.get_default_playback_device() {
            Ok(Some(dev)) => dev,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        let device_id = default_device.device_id;
        let device_name = default_device.device_name;
        let now_elapsed = self.clock.elapsed();

        if let Some(last) = &self.last_applied
            && last.device_id == device_id
            && now_elapsed.saturating_sub(last.applied_time) < self.same_device_suppress_window
        {
            return Ok(None);
        }

        let profiles = self.store.load().map_err(AudioError::Io)?;
        let name_lookup = if device_name == "(unknown)" {
            None
        } else {
            Some(device_name.as_str())
        };
        let matched = match_profile(&profiles, &device_id, name_lookup);

        if let Some(profile) = matched {
            self.adapter
                .set_master_volume(Some(&device_id), profile.master_volume)?;
            self.adapter.set_mute(Some(&device_id), profile.is_muted)?;

            self.last_applied = Some(AppliedState {
                device_id: device_id.clone(),
                applied_time: now_elapsed,
            });

            let event = ProfileAppliedEvent {
                device_id,
                device_name,
                master_volume: profile.master_volume,
                is_muted: profile.is_muted,
                is_new_profile: false,
            };

            self.applied_events.push(event.clone());
            Ok(Some(event))
        } else {
            let vol = self.adapter.get_master_volume(Some(&device_id))?;
            let mute = self.adapter.get_mute(Some(&device_id))?;
            let now = self.clock.now();

            let new_profile = VolumeProfile {
                device_id: device_id.clone(),
                device_name: if device_name == "(unknown)" {
                    String::new()
                } else {
                    device_name.clone()
                },
                master_volume: vol,
                is_muted: mute,
                created_at: now,
                last_applied: now,
            };

            self.store.save(&new_profile).map_err(AudioError::Io)?;

            self.last_applied = Some(AppliedState {
                device_id: device_id.clone(),
                applied_time: now_elapsed,
            });

            let event = ProfileAppliedEvent {
                device_id,
                device_name,
                master_volume: vol,
                is_muted: mute,
                is_new_profile: true,
            };

            self.applied_events.push(event.clone());
            Ok(Some(event))
        }
    }

    pub fn drain_applied_events(&mut self) -> Vec<ProfileAppliedEvent> {
        std::mem::take(&mut self.applied_events)
    }

    pub fn stop(&mut self) {
        self.stopped = true;
        self.pending_settle_due = None;
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
}

pub struct DeviceMonitorOrchestrator<
    A: AudioDeviceAdapter + 'static,
    S: ProfileStore + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
> {
    engine: Arc<Mutex<OrchestratorEngine<A, S, C>>>,
    monitor_service: Option<DeviceMonitorService>,
    worker_handle: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
    event_callback: CallbackSlot,
}

impl<
    A: AudioDeviceAdapter + 'static,
    S: ProfileStore + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
> DeviceMonitorOrchestrator<A, S, C>
{
    pub fn new(
        adapter: Arc<A>,
        store: Arc<S>,
        clock: Arc<C>,
        debounce_window: Option<Duration>,
        same_device_suppress_window: Option<Duration>,
    ) -> Self {
        let engine = OrchestratorEngine::new(
            adapter,
            store,
            clock,
            debounce_window,
            same_device_suppress_window,
        );
        Self {
            engine: Arc::new(Mutex::new(engine)),
            monitor_service: None,
            worker_handle: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
            event_callback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_on_profile_applied<F>(&self, callback: F)
    where
        F: Fn(ProfileAppliedEvent) + Send + Sync + 'static,
    {
        let mut cb_guard = self.event_callback.lock().unwrap();
        *cb_guard = Some(Box::new(callback));
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        if self.worker_handle.is_some() {
            return Ok(());
        }

        let (tx, rx) = mpsc::channel();
        let monitor = DeviceMonitorService::start(tx)?;
        self.monitor_service = Some(monitor);

        let engine_clone = Arc::clone(&self.engine);
        let stop_signal_clone = Arc::clone(&self.stop_signal);
        let callback_clone = Arc::clone(&self.event_callback);

        stop_signal_clone.store(false, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            let poll_interval = Duration::from_millis(50);
            while !stop_signal_clone.load(Ordering::SeqCst) {
                while let Ok(msg) = rx.try_recv() {
                    let mut engine = engine_clone.lock().unwrap();
                    if let Ok(Some(event)) = engine.on_device_changed(msg) {
                        let cb_lock = callback_clone.lock().unwrap();
                        if let Some(cb) = cb_lock.as_ref() {
                            cb(event);
                        }
                    }
                }

                {
                    let mut engine = engine_clone.lock().unwrap();
                    if let Ok(events) = engine.advance_time() {
                        let cb_lock = callback_clone.lock().unwrap();
                        if let Some(cb) = cb_lock.as_ref() {
                            for event in events {
                                cb(event);
                            }
                        }
                    }
                }

                thread::sleep(poll_interval);
            }
        });

        self.worker_handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        self.monitor_service.take(); // Unregisters COM callback

        {
            let mut engine = self.engine.lock().unwrap();
            engine.stop();
        }

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl<
    A: AudioDeviceAdapter + 'static,
    S: ProfileStore + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
> Drop for DeviceMonitorOrchestrator<A, S, C>
{
    fn drop(&mut self) {
        self.stop();
    }
}
