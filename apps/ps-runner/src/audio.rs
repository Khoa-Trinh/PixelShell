use windows::Win32::{
    Media::Audio::{
        eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL},
};

pub struct AudioSystem {}

impl AudioSystem {
    pub fn new() -> Self {
        unsafe {
            let _ = CoInitialize(None);
        }
        Self {}
    }

    pub fn get_volume_control(&self) -> Option<IAudioEndpointVolume> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;

            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            device.Activate(CLSCTX_ALL, None).ok()
        }
    }
}

impl Drop for AudioSystem {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
