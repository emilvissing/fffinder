//! The images the large previews show: in the gallery, in the column view's
//! and the preview pane's card, and in Quick Look. GPUI keeps every image
//! that `img()` loads from a path for as long as the app runs, decoded at its
//! full size, which over a folder of photos comes to gigabytes. So a browser
//! loads them into a cache of its own, and lets go of all but the last few
//! it showed.

use fffinder_core::FileEntry;
use gpui_kit::{
    App, AppContext as _, Entity, ImageCacheError, ImageCacheItem, ImageSource, RenderImage,
    Resource, Window,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// How many images a browser keeps. It shows one at a time, the same one in
/// its window and in Quick Look, so the others are the ones it showed just
/// before, kept so that going back to them doesn't decode them again.
const KEEP: usize = 3;

/// An image file and when it was last modified, so that a file that has
/// changed is loaded again.
type Key = (PathBuf, Option<SystemTime>);

/// The images a browser's large previews hold, the one shown last at the end.
pub(super) struct PreviewImages(VecDeque<(Key, ImageCacheItem)>);

impl PreviewImages {
    pub(super) fn new(cx: &mut App) -> Entity<PreviewImages> {
        let images = cx.new(|_| PreviewImages(VecDeque::new()));
        // A browser that closes takes its images' textures with it.
        cx.observe_release(&images, |images, cx| {
            for (_, item) in images.0.drain(..) {
                if let Some(Ok(image)) = item.get() {
                    cx.drop_image(image, None);
                }
            }
        })
        .detach();
        images
    }

    /// `entry`'s image for `img()`, kept in `images`. It loads in the
    /// background; until then, and when the file can't be decoded, the
    /// image's loading and fallback elements show.
    pub(super) fn source(images: &Entity<PreviewImages>, entry: &FileEntry) -> ImageSource {
        let (images, key) = (images.clone(), (entry.path.clone(), entry.modified));
        ImageSource::Custom(Arc::new(move |window, cx| {
            images.update(cx, |images, cx| images.load(&key, window, cx))
        }))
    }

    /// The image for `key`, which starts loading when it isn't held, in the
    /// place of the one shown longest ago. `None` while it loads; the view
    /// being drawn is drawn again when it has.
    fn load(
        &mut self,
        key: &Key,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        let at = self.0.iter().position(|(held, _)| held == key);
        if let Some(shown) = at.and_then(|at| self.0.remove(at)) {
            self.0.push_back(shown);
        } else {
            // What's held of the same file is the file as it was before it
            // changed, which won't be shown again.
            let at = self.0.iter().position(|(held, _)| held.0 == key.0);
            if let Some((_, changed)) = at.and_then(|at| self.0.remove(at)) {
                Self::let_go(changed, window, cx);
            }
            let item = ImageCacheItem::new(&Resource::Path(key.0.as_path().into()), cx);
            self.0.push_back((key.clone(), item));
            if self.0.len() > KEEP
                && let Some((_, oldest)) = self.0.pop_front()
            {
                Self::let_go(oldest, window, cx);
            }
        }
        let (_, shown) = self.0.back()?;
        shown.use_image(window)
    }

    /// Free `item`'s texture in every window that drew it. Dropping the item
    /// frees the pixels, and stops it loading if it still was.
    fn let_go(item: ImageCacheItem, window: &mut Window, cx: &mut App) {
        if let Some(Ok(image)) = item.get() {
            cx.drop_image(image, Some(window));
        }
    }

    /// The files whose images are held, the one shown last at the end, each
    /// with its image once that has loaded.
    pub(super) fn held(&self) -> Vec<(PathBuf, Option<Arc<RenderImage>>)> {
        self.0
            .iter()
            .map(|((path, _), item)| (path.clone(), item.get().and_then(Result::ok)))
            .collect()
    }
}
