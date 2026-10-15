//! Apercu PDF via le moteur de Windows (Windows.Data.Pdf) : une page rendue en image.

use std::path::Path;
use std::sync::Arc;

use gpui::RenderImage;
use image::Frame;
use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
use windows::Storage::StorageFile;
use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::HSTRING;

fn render(path: &Path, index: u32, width: u32) -> Result<(Arc<RenderImage>, u32), String> {
    let full = std::fs::canonicalize(path).map_err(|e| format!("chemin : {e}"))?;
    // canonicalize ajoute le prefixe \?\ que StorageFile n'accepte pas.
    let lossy = full.to_string_lossy().into_owned();
    let text = lossy.strip_prefix(r"\\?\").unwrap_or(&lossy).to_string();
    let err = |what: &str| {
        let what = what.to_string();
        move |e: windows::core::Error| format!("{what} : {e}")
    };
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(text.as_str()))
        .map_err(err("fichier"))?
        .get()
        .map_err(|e| format!("fichier.get [{text}] : {e}"))?;
    let doc = PdfDocument::LoadFromFileAsync(&file).map_err(err("pdf"))?.get().map_err(err("pdf.get"))?;
    let count = doc.PageCount().map_err(err("pages"))?;
    if count == 0 {
        return Err("aucune page".into());
    }
    let page = doc.GetPage(index.min(count - 1)).map_err(err("page"))?;
    let stream = InMemoryRandomAccessStream::new().map_err(err("flux"))?;
    let opts = PdfPageRenderOptions::new().map_err(err("options"))?;
    opts.SetDestinationWidth(width).map_err(err("largeur"))?;
    page.RenderWithOptionsToStreamAsync(&stream, &opts).map_err(err("rendu"))?.get().map_err(err("rendu.get"))?;
    let size = stream.Size().map_err(err("taille"))? as u32;
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).map_err(err("lecture"))?).map_err(err("reader"))?;
    reader.LoadAsync(size).map_err(err("load"))?.get().map_err(err("load.get"))?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).map_err(err("octets"))?;
    let mut rgba = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| format!("decodage : {e}"))?
        .to_rgba8();
    // Fond blanc sous la transparence (les PDF sont rendus sur transparent), puis BGRA pour GPUI.
    for p in rgba.chunks_exact_mut(4) {
        let a = p[3] as u32;
        for c in &mut p[..3] {
            *c = ((*c as u32 * a + 255 * (255 - a)) / 255) as u8;
        }
        p[3] = 255;
        p.swap(0, 2);
    }
    Ok((Arc::new(RenderImage::new(vec![Frame::new(rgba)])), count))
}

/// Rend la page `index` (0 = premiere) a la largeur `width` px. Retourne l'image et le nombre de pages.
/// Bloquant : a appeler hors du thread d'interface.
pub fn render_page(path: &Path, index: u32, width: u32) -> Option<(Arc<RenderImage>, u32)> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    match render(path, index, width) {
        Ok(r) => Some(r),
        Err(_) => {
            None
        }
    }
}
