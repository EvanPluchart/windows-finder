//! Lecture video dans l'apercu : Windows.Media.Playback.MediaPlayer en mode "frame server" (image + son synchronises),
//! les images sont recopiees dans un tampon BGRA que l'interface affiche.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use windows::Foundation::TypedEventHandler;
use windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use windows::Media::Core::MediaSource;
use windows::Media::Playback::{IMediaPlaybackSource, MediaPlayer};
use windows::Storage::StorageFile;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11SurfaceFromDXGISurface;
use windows::core::{HSTRING, IInspectable, Interface};

pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// BGRA, `width * height * 4` octets.
    pub data: Vec<u8>,
}

struct Shared {
    frame: Mutex<Option<Frame>>,
    dirty: AtomicBool,
}

struct Gpu {
    device: ID3D11Device,
    ctx: ID3D11DeviceContext,
    size: (u32, u32),
    texture: Option<ID3D11Texture2D>,
    staging: Option<ID3D11Texture2D>,
    surface: Option<IDirect3DSurface>,
}

impl Gpu {
    fn ensure(&mut self, w: u32, h: u32) -> windows::core::Result<()> {
        if self.size == (w, h) && self.texture.is_some() {
            return Ok(());
        }
        let mut desc = D3D11_TEXTURE2D_DESC {
            Width: w,
            Height: h,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut tex = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut tex))? };
        let tex = tex.ok_or_else(windows::core::Error::empty)?;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let mut staging = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut staging))? };
        let dxgi: IDXGISurface = tex.cast()?;
        let inspectable = unsafe { CreateDirect3D11SurfaceFromDXGISurface(&dxgi)? };
        self.surface = Some(inspectable.cast::<IDirect3DSurface>()?);
        self.texture = Some(tex);
        self.staging = staging;
        self.size = (w, h);
        Ok(())
    }
}

/// Les objets D3D11 sont utilisables depuis n'importe quel thread ; le contexte reste protege par le Mutex.
struct GpuBox(Mutex<Gpu>);
unsafe impl Send for GpuBox {}
unsafe impl Sync for GpuBox {}

pub struct Video {
    player: MediaPlayer,
    shared: Arc<Shared>,
}

impl Video {
    /// Ouvre `path` et commence la lecture.
    pub fn open(path: &Path, autoplay: bool) -> Result<Self, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let full = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        let lossy = full.to_string_lossy().into_owned();
        let text = lossy.strip_prefix(r"\\?\").unwrap_or(&lossy).to_string();
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(text))
            .and_then(|a| a.get())
            .map_err(|e| format!("fichier : {e}"))?;
        let source = MediaSource::CreateFromStorageFile(&file).map_err(|e| e.to_string())?;

        let mut device = None;
        let mut ctx = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut ctx),
            )
            .map_err(|e| format!("d3d : {e}"))?;
        }
        let (device, ctx) = (device.ok_or("pas de device d3d")?, ctx.ok_or("pas de contexte d3d")?);
        let gpu = GpuBox(Mutex::new(Gpu { device, ctx, size: (0, 0), texture: None, staging: None, surface: None }));
        let shared = Arc::new(Shared { frame: Mutex::new(None), dirty: AtomicBool::new(false) });

        let player = MediaPlayer::new().map_err(|e| e.to_string())?;
        player.SetIsVideoFrameServerEnabled(true).map_err(|e| e.to_string())?;
        let sh = shared.clone();
        player
            .VideoFrameAvailable(&TypedEventHandler::<MediaPlayer, IInspectable>::new(move |sender, _| {
                let gpu = &gpu; // capture la boite entiere (Send + Sync), pas seulement son champ
                let Some(sender) = sender.as_ref() else { return Ok(()) };
                let session = sender.PlaybackSession()?;
                let (w, h) = (session.NaturalVideoWidth()?, session.NaturalVideoHeight()?);
                if w == 0 || h == 0 {
                    return Ok(());
                }
                let mut g = gpu.0.lock().unwrap();
                g.ensure(w, h)?;
                let (Some(surface), Some(tex), Some(staging)) = (g.surface.clone(), g.texture.clone(), g.staging.clone()) else {
                    return Ok(());
                };
                sender.CopyFrameToVideoSurface(&surface)?;
                unsafe {
                    g.ctx.CopyResource(&staging, &tex);
                    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                    g.ctx.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
                    let mut data = vec![0u8; (w * h * 4) as usize];
                    let row = (w * 4) as usize;
                    for y in 0..h as usize {
                        let src = (mapped.pData as *const u8).add(y * mapped.RowPitch as usize);
                        std::ptr::copy_nonoverlapping(src, data.as_mut_ptr().add(y * row), row);
                    }
                    g.ctx.Unmap(&staging, 0);
                    *sh.frame.lock().unwrap() = Some(Frame { width: w, height: h, data });
                }
                sh.dirty.store(true, Ordering::Release);
                Ok(())
            }))
            .map_err(|e| e.to_string())?;
        let src: IMediaPlaybackSource = source.cast().map_err(|e| e.to_string())?;
        player.SetSource(&src).map_err(|e| e.to_string())?;
        if autoplay {
            player.Play().map_err(|e| e.to_string())?;
        }
        Ok(Self { player, shared })
    }

    /// Derniere image decodee, si une nouvelle est arrivee depuis le dernier appel.
    pub fn take_frame(&self) -> Option<Frame> {
        if self.shared.dirty.swap(false, Ordering::AcqRel) {
            self.shared.frame.lock().unwrap().take()
        } else {
            None
        }
    }

    pub fn toggle(&self) {
        let Ok(session) = self.player.PlaybackSession() else { return };
        if self.is_playing() {
            let _ = self.player.Pause();
        } else {
            let _ = self.player.Play();
        }
        let _ = session;
    }

    pub fn is_playing(&self) -> bool {
        self.player
            .PlaybackSession()
            .and_then(|s| s.PlaybackState())
            .map(|st| st.0 == 3) // MediaPlaybackState::Playing
            .unwrap_or(false)
    }

    /// Position et duree en secondes.
    pub fn times(&self) -> (f64, f64) {
        let Ok(s) = self.player.PlaybackSession() else { return (0.0, 0.0) };
        let secs = |t: windows::Foundation::TimeSpan| t.Duration as f64 / 10_000_000.0;
        (s.Position().map(secs).unwrap_or(0.0), s.NaturalDuration().map(secs).unwrap_or(0.0))
    }

    pub fn set_volume(&self, v: f64) {
        let _ = self.player.SetVolume(v.clamp(0.0, 1.0));
    }

    pub fn set_muted(&self, m: bool) {
        let _ = self.player.SetIsMuted(m);
    }

    pub fn seek(&self, seconds: f64) {
        if let Ok(s) = self.player.PlaybackSession() {
            let _ = s.SetPosition(windows::Foundation::TimeSpan { Duration: (seconds * 10_000_000.0) as i64 });
        }
    }
}

impl Drop for Video {
    fn drop(&mut self) {
        let _ = self.player.Pause();
        let _ = self.player.Close();
    }
}
