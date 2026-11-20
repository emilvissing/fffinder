//! Thumbnails of files: images, PDF pages, video frames, office documents,
//! and fonts. Images in the common formats are scaled down here, the same on
//! every system. The system draws the rest: macOS with QuickLook, Windows
//! with the shell's thumbnail handlers (Explorer's), and Linux with the
//! freedesktop.org thumbnailers its desktop installed. They're cached as PNGs
//! in the data folder and loaded as GPUI assets, so a view shows the file's
//! icon until its thumbnail is ready and then redraws. A type nothing draws
//! keeps its icon.

use fffinder_core::{FileEntry, FileKind};
use gpui_kit::{
    App, AppContext as _, Asset, ImageCacheError, ImageSource, ImgResourceLoader, Resource,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// The most the cached thumbnails may take; the least recently used go
/// first when they take more.
const CACHE_LIMIT: u64 = 256 * 1024 * 1024;

/// Image files bigger than this aren't decoded here: the system draws them,
/// where it can.
pub const IMAGE_LIMIT: u64 = 40_000_000;

/// What to draw and how big: the file, its modification time (so an edited
/// file gets a new thumbnail), and the size in pixels.
type Key = (PathBuf, Option<SystemTime>, u32);

/// Types with a useful picture: everything that isn't plain text, code, a
/// folder, or an app, which keep their icons.
pub fn wanted(entry: &FileEntry) -> bool {
    if entry.is_dir {
        return false;
    }
    let ext = entry
        .path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match entry.kind {
        FileKind::Pdf | FileKind::Video | FileKind::Image => true,
        FileKind::Text | FileKind::Data | FileKind::Other => matches!(
            ext.as_str(),
            "doc"
                | "docx"
                | "xls"
                | "xlsx"
                | "ppt"
                | "pptx"
                | "pages"
                | "numbers"
                | "key"
                | "rtf"
                | "odt"
                | "ods"
                | "odp"
                | "epub"
                | "ttf"
                | "otf"
                | "ttc"
                | "psd"
                | "ai"
                | "eps"
                | "usdz"
                | "stl"
                | "obj"
        ),
        FileKind::Folder
        | FileKind::Application
        | FileKind::Audio
        | FileKind::Archive
        | FileKind::Code
        | FileKind::Executable => false,
    }
}

/// An image of `entry`'s thumbnail at `px` pixels across, for `img()`. It
/// loads in the background; until then, and when there is none, the
/// image's loading and fallback elements show.
pub fn image(entry: &FileEntry, px: u32) -> ImageSource {
    let key: Key = (entry.path.clone(), entry.modified, px);
    ImageSource::Custom(Arc::new(move |window, cx| {
        // UI tests run on a deterministic executor that our worker threads
        // would wake from outside, so they keep the icons.
        if cx.has_global::<crate::system::Recorder>() {
            return Some(Err(ImageCacheError::Other(Arc::new(anyhow::anyhow!(
                "no thumbnails in tests"
            )))));
        }
        match window.use_asset::<Thumbnail>(&key, cx)? {
            Some(png) => window.use_asset::<ImgResourceLoader>(&Resource::Path(png.into()), cx),
            None => Some(Err(ImageCacheError::Other(Arc::new(anyhow::anyhow!(
                "no thumbnail"
            ))))),
        }
    }))
}

/// The thumbnail PNG for a [`Key`], made once and then read from disk.
enum Thumbnail {}

impl Asset for Thumbnail {
    type Source = Key;
    type Output = Option<PathBuf>;

    fn load(
        key: Key,
        cx: &mut App,
    ) -> impl std::future::Future<Output = Option<PathBuf>> + Send + 'static {
        let (reply, answer) = async_channel::bounded(1);
        let sent = queue().send_blocking(Job { key, reply }).is_ok();
        cx.background_spawn(async move {
            if sent {
                answer.recv().await.ok().flatten()
            } else {
                None
            }
        })
    }
}

struct Job {
    key: Key,
    reply: async_channel::Sender<Option<PathBuf>>,
}

/// QuickLook does its work elsewhere and calls back, so a few threads of
/// our own wait on it instead of tying up GPUI's background threads. The
/// cache is trimmed before any of them makes a thumbnail, and not beside
/// them: trimming removes unfinished PNGs, one of which a thread could be
/// writing, and the PNGs used longest ago, one of which a thread could
/// just have handed out. Either would leave an item on its icon.
fn queue() -> &'static async_channel::Sender<Job> {
    static QUEUE: OnceLock<async_channel::Sender<Job>> = OnceLock::new();
    static TRIMMED: std::sync::Once = std::sync::Once::new();
    QUEUE.get_or_init(|| {
        let (tx, rx) = async_channel::unbounded::<Job>();
        for n in 0..3 {
            let rx = rx.clone();
            let _ = std::thread::Builder::new()
                .name(format!("thumbnails-{n}"))
                .spawn(move || {
                    // The thread that gets here first trims, and the others
                    // wait for it.
                    TRIMMED.call_once(|| prune(&cache_dir(), CACHE_LIMIT));
                    while let Ok(job) = rx.recv_blocking() {
                        let _ = job.reply.send_blocking(cached(&cache_dir(), &job.key));
                    }
                });
        }
        tx
    })
}

fn cache_dir() -> PathBuf {
    fffinder_core::config::data_dir().join("thumbnails")
}

/// A name for `key`'s PNG that stays the same from one build of fffinder
/// to the next: FNV-1a over the path's bytes, the modification time, and
/// the size, unlike std's hasher, which may change with Rust.
fn file_name(key: &Key) -> String {
    let (path, modified, px) = key;
    let nanos = modified
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let (nanos, px) = (nanos.to_le_bytes(), px.to_le_bytes());
    let bytes = path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .chain(&nanos)
        .chain(&px);
    let hash = bytes.fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}.png")
}

/// The PNG for `key` in the cache folder `dir`, made now if it isn't cached
/// yet.
fn cached(dir: &Path, key: &Key) -> Option<PathBuf> {
    let png = dir.join(file_name(key));
    if png.is_file() {
        // Marked as used, so trimming the cache keeps it.
        let _ = std::fs::File::options()
            .write(true)
            .open(&png)
            .and_then(|file| file.set_modified(SystemTime::now()));
        return Some(png);
    }
    std::fs::create_dir_all(dir).ok()?;
    let partial = png.with_extension("partial.png");
    // A malformed image must not take its thumbnail thread down with it.
    let scaled = decodes(&key.0)
        && std::panic::catch_unwind(|| make_image(&key.0, key.2, &partial))
            .is_ok_and(|made| made.is_some());
    // The system may still draw an image that couldn't be decoded here.
    if !scaled {
        make(&key.0, key.2, &partial)?;
    }
    std::fs::rename(&partial, &png).ok()?;
    Some(png)
}

/// Whether `file` is, by its name, an image in a format that's decoded here
/// and not by the system: the common ones, which then look the same on every
/// system.
fn decodes(file: &Path) -> bool {
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "tif" | "tiff"
    )
}

/// Save the image `file`, scaled down to fit in `px` by `px` pixels, as the
/// PNG `out`. It's turned the way its EXIF orientation says, as GPUI turns
/// an image it shows whole. `None` for a file over [`IMAGE_LIMIT`], and for
/// one that can't be decoded.
fn make_image(file: &Path, px: u32, out: &Path) -> Option<()> {
    use image::ImageDecoder as _;
    if std::fs::metadata(file).ok()?.len() > IMAGE_LIMIT {
        return None;
    }
    let mut decoder = image::ImageReader::open(file)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    // A decoder keeps what it allocates itself within the limits. The
    // picture it decodes into isn't counted unless it's reserved here.
    let mut limits = image::Limits::default();
    limits.reserve(decoder.total_bytes()).ok()?;
    decoder.set_limits(limits).ok()?;
    let orientation = decoder.orientation().ok()?;
    // Turned once it's small, which gives the same picture as turning it
    // first and moves far fewer pixels.
    let mut image = shrink(image::DynamicImage::from_decoder(decoder).ok()?, px);
    image.apply_orientation(orientation);
    // Eight bits a channel is all a thumbnail needs, and every kind of
    // picture converts to it, where not every kind can be saved as a PNG.
    image
        .into_rgba8()
        .save_with_format(out, image::ImageFormat::Png)
        .ok()
}

/// `image` scaled down to fit in `px` by `px` pixels, in its proportions.
/// One that fits already stays as it is, where scaling would blow it up.
fn shrink(image: image::DynamicImage, px: u32) -> image::DynamicImage {
    if image.width() <= px && image.height() <= px {
        return image;
    }
    if !image.color().has_alpha() {
        return image.thumbnail(px, px);
    }
    // Scaling down averages pixels, and a see-through pixel's color has to
    // count for as little as shows of it: what's fully transparent is often
    // black underneath, and would darken every edge beside it. So colors
    // are multiplied by their alpha to be averaged, and divided by it after.
    let mut image = image.into_rgba8();
    for image::Rgba([red, green, blue, alpha]) in image.pixels_mut() {
        for color in [red, green, blue] {
            *color = ((u16::from(*color) * u16::from(*alpha) + 127) / 255) as u8;
        }
    }
    let image = image::DynamicImage::ImageRgba8(image).thumbnail(px, px);
    let mut image = image.into_rgba8();
    for image::Rgba([red, green, blue, alpha]) in image.pixels_mut() {
        for color in [red, green, blue] {
            if *alpha > 0 {
                *color = (u16::from(*color) * 255 / u16::from(*alpha)).min(255) as u8;
            }
        }
    }
    image::DynamicImage::ImageRgba8(image)
}

/// Remove unfinished PNGs left by a crash, and the least recently used
/// ones while the folder holds more than `limit` bytes of them.
fn prune(dir: &Path, limit: u64) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut pngs: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".partial.png") {
            let _ = std::fs::remove_file(&path);
        } else if name.ends_with(".png")
            && let Ok(meta) = entry.metadata()
        {
            let used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            pngs.push((used, meta.len(), path));
        }
    }
    let mut total: u64 = pngs.iter().map(|(_, len, _)| len).sum();
    if total <= limit {
        return;
    }
    pngs.sort_unstable_by_key(|(used, _, _)| *used);
    for (_, len, path) in pngs {
        if total <= limit {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
        }
    }
}

#[cfg(target_os = "macos")]
fn make(file: &Path, px: u32, out: &Path) -> Option<()> {
    crate::macos::save_thumbnail(file, px, out).ok()
}

#[cfg(windows)]
fn make(file: &Path, px: u32, out: &Path) -> Option<()> {
    crate::win::save_thumbnail(file, px, out).ok()
}

/// Run the desktop's thumbnailer for the file's type, which writes the PNG.
/// One that takes longer than [`THUMBNAILER_TIMEOUT`] is stopped, so a hung
/// one can't hold a thumbnail thread for good.
#[cfg(not(any(target_os = "macos", windows)))]
fn make(file: &Path, px: u32, out: &Path) -> Option<()> {
    use fffinder_core::thumbnailers::{self, Thumbnailer};
    use wait_timeout::ChildExt as _;
    const THUMBNAILER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
    static INSTALLED: OnceLock<Vec<Thumbnailer>> = OnceLock::new();
    let installed = INSTALLED.get_or_init(|| thumbnailers::installed(&thumbnailers::data_dirs()));
    let mime = crate::system::mime_type(file)?;
    let thumbnailer = thumbnailers::for_mime(installed, &mime)?;
    let words = thumbnailers::command(&thumbnailer.exec, file, out, px);
    let (program, args) = words.split_first()?;
    let mut child = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let status = match child.wait_timeout(THUMBNAILER_TIMEOUT).ok()? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    (status.success() && out.is_file()).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_stay_the_same() {
        let key: Key = (
            PathBuf::from("/a/b.pdf"),
            Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1)),
            128,
        );
        assert_eq!(file_name(&key), file_name(&key.clone()));
        assert_eq!(file_name(&key), "94ce6be0f59c52a1.png");
        assert_ne!(file_name(&key), file_name(&(key.0.clone(), key.1, 256)));
    }

    #[test]
    fn prunes_the_least_recently_used() {
        let dir = std::env::temp_dir().join(format!("fffinder-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let at = |secs| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        for (name, used) in [("old.png", 1), ("mid.png", 2), ("new.png", 3)] {
            std::fs::write(dir.join(name), [0u8; 10]).unwrap();
            std::fs::File::options()
                .write(true)
                .open(dir.join(name))
                .unwrap()
                .set_modified(at(used))
                .unwrap();
        }
        std::fs::write(dir.join("x.partial.png"), [0u8; 1]).unwrap();
        prune(&dir, 20);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["mid.png", "new.png"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An empty folder for one test's images.
    fn image_dir(test: &str) -> PathBuf {
        let name = format!("fffinder-thumbs-{test}-{}", std::process::id());
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_common_image_formats_are_decoded_here() {
        for name in [
            "a.png", "b.jpg", "c.JPEG", "d.gif", "e.webp", "f.bmp", "g.ico", "h.tif", "i.TIFF",
        ] {
            assert!(decodes(Path::new(name)), "{name}");
        }
        for name in ["a.svg", "b.heic", "c.psd", "d.pdf", "e.mp4", "png"] {
            assert!(!decodes(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn an_image_of_each_format_is_scaled_down_to_fit() {
        use image::DynamicImage;
        let dir = image_dir("formats");
        let out = dir.join("out.png");
        for (name, image) in [
            ("a.png", DynamicImage::new_rgba8(256, 128)),
            ("b.jpg", DynamicImage::new_rgb8(256, 128)),
            ("c.gif", DynamicImage::new_rgba8(256, 128)),
            ("d.webp", DynamicImage::new_rgba8(256, 128)),
            ("e.bmp", DynamicImage::new_rgb8(256, 128)),
            ("f.ico", DynamicImage::new_rgba8(256, 128)),
            // Sixteen bits a channel, which the thumbnail doesn't keep.
            ("g.tiff", DynamicImage::new_rgb16(256, 128)),
        ] {
            let file = dir.join(name);
            image.save(&file).unwrap();
            make_image(&file, 128, &out).unwrap_or_else(|| panic!("no thumbnail of {name}"));
            let thumbnail = image::open(&out).unwrap();
            assert_eq!((thumbnail.width(), thumbnail.height()), (128, 64), "{name}");
            assert_eq!(thumbnail.color(), image::ColorType::Rgba8, "{name}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_small_image_isnt_scaled_up() {
        let dir = image_dir("small");
        let (file, out) = (dir.join("small.png"), dir.join("out.png"));
        image::RgbaImage::new(20, 10).save(&file).unwrap();
        make_image(&file, 128, &out).expect("a thumbnail");
        assert_eq!(image::image_dimensions(&out).unwrap(), (20, 10));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_is_see_through_doesnt_darken_the_edge_beside_it() {
        // Red beside nothing at all, which is black underneath. The edge
        // is between two pixels that are averaged into one.
        let image = image::RgbaImage::from_fn(256, 16, |x, _| match x < 129 {
            true => image::Rgba([255, 0, 0, 255]),
            false => image::Rgba([0, 0, 0, 0]),
        });
        let small = shrink(image::DynamicImage::ImageRgba8(image), 128).into_rgba8();
        assert_eq!(small.dimensions(), (128, 8));
        assert_eq!(small.get_pixel(63, 4).0, [255, 0, 0, 255], "red");
        let [red, green, blue, alpha] = small.get_pixel(64, 4).0;
        assert_eq!((red, green, blue), (255, 0, 0), "as red at the edge");
        assert!((127..=128).contains(&alpha), "and half there: {alpha}");
        assert_eq!(small.get_pixel(65, 4).0, [0, 0, 0, 0], "nothing");
    }

    #[test]
    fn a_file_that_is_no_image_gets_no_thumbnail() {
        let dir = image_dir("garbage");
        let (file, out) = (dir.join("broken.png"), dir.join("out.png"));
        std::fs::write(&file, "not a picture at all").unwrap();
        assert!(make_image(&file, 128, &out).is_none());
        assert!(make_image(&dir.join("missing.png"), 128, &out).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_image_file_over_the_limit_gets_no_thumbnail() {
        let dir = image_dir("limit");
        let (file, out) = (dir.join("huge.png"), dir.join("out.png"));
        image::RgbaImage::new(20, 10).save(&file).unwrap();
        // What follows a PNG's end is ignored, so the file stays an image
        // however long it's made.
        let resize = |len| {
            let file = std::fs::File::options().write(true).open(&file).unwrap();
            file.set_len(len).unwrap();
        };
        resize(IMAGE_LIMIT);
        assert!(make_image(&file, 128, &out).is_some(), "at the limit");
        resize(IMAGE_LIMIT + 1);
        assert!(make_image(&file, 128, &out).is_none(), "over the limit");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_image_is_scaled_down_into_the_cache_and_read_from_it_after() {
        let dir = image_dir("cache");
        let (file, cache) = (dir.join("photo.png"), dir.join("thumbnails"));
        image::RgbaImage::new(256, 128).save(&file).unwrap();
        let key: Key = (file.clone(), None, 64);
        let png = cached(&cache, &key).expect("a thumbnail");
        assert_eq!(png, cache.join(file_name(&key)));
        assert_eq!(image::image_dimensions(&png).unwrap(), (64, 32));
        let left: Vec<String> = std::fs::read_dir(&cache)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, [file_name(&key)], "nothing unfinished is left");

        // The image isn't read again: it can be gone, and the thumbnail
        // made of it is still the one that's shown.
        std::fs::remove_file(&file).unwrap();
        assert_eq!(cached(&cache, &key), Some(png));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_photo_stored_on_its_side_is_turned_upright() {
        let dir = image_dir("orientation");
        let (file, out) = (dir.join("photo.jpg"), dir.join("out.png"));
        // Red on the left and blue on the right, as the camera stored it.
        let stored = image::RgbImage::from_fn(240, 120, |x, _| match x < 120 {
            true => image::Rgb([255, 0, 0]),
            false => image::Rgb([0, 0, 255]),
        });
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
            .encode_image(&stored)
            .unwrap();
        // EXIF that says to turn the picture a quarter clockwise to show it
        // (orientation 6): an APP1 segment holding a little-endian TIFF
        // header and the one tag, 0x0112, put right after the file's start.
        let mut exif = vec![0xFF, 0xE1, 0, 34];
        exif.extend_from_slice(b"Exif\0\0II*\0\x08\0\0\0\x01\0");
        exif.extend_from_slice(&[0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0]);
        jpeg.splice(2..2, exif);
        std::fs::write(&file, jpeg).unwrap();

        make_image(&file, 64, &out).expect("a thumbnail");
        let thumbnail = image::open(&out).unwrap().into_rgb8();
        assert_eq!(thumbnail.dimensions(), (32, 64), "upright");
        let [red, _, blue] = thumbnail.get_pixel(16, 12).0;
        assert!(red > 200 && blue < 60, "red at the top: {red} {blue}");
        let [red, _, blue] = thumbnail.get_pixel(16, 52).0;
        assert!(red < 60 && blue > 200, "blue at the bottom: {red} {blue}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn quicklook_draws_a_pdf() {
        let dir = std::env::temp_dir().join(format!("fffinder-thumbs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("page.pdf");
        std::fs::write(
            &pdf,
            "%PDF-1.1\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\n\
             trailer << /Root 1 0 R >>\n%%EOF\n",
        )
        .unwrap();
        let out = dir.join("thumb.png");
        make(&pdf, 128, &out).expect("QuickLook made a thumbnail");
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"), "a PNG");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
