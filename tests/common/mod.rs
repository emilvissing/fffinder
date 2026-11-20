//! Shared harness for the UI tests: a fixture folder, a window driven through
//! real mouse and keyboard events, and a watchdog that fails the run when a
//! step hangs.

#![allow(dead_code)]

use fffinder::finder::{AppState, Finder, PickerOptions};
use fffinder::model::Location;
use fffinder::system::{Recorder, SystemCall};
use fffinder_core::terminals::Terminal;
use fffinder_core::{Appearance, Engine, EngineOptions, SidebarMode};
use gpui_kit::base::test_support::{self, ElementSnapshot};
use gpui_kit::component::WindowExt as _;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Context, ElementId, Entity, FileDropEvent,
    InputEvent as _, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    TestAppContext, Window, WindowOptions, point, px,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

/// A UI step that runs longer than this is treated as a hang.
const HANG_LIMIT: Duration = Duration::from_secs(20);

pub struct Fixture {
    pub dir: PathBuf,
}

impl Fixture {
    /// ```text
    /// Budget.csv  Projects/atlas/Project Atlas.txt  Roadmap.md  notes.txt  .secret
    /// ```
    pub fn new(name: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("fffinder-ui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Projects/atlas")).unwrap();
        std::fs::write(dir.join("Budget.csv"), "a,b\n1,2\n").unwrap();
        std::fs::write(dir.join("Roadmap.md"), "# Roadmap\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "plain notes\n").unwrap();
        std::fs::write(dir.join(".secret"), "hidden").unwrap();
        std::fs::write(dir.join("Projects/atlas/Project Atlas.txt"), "atlas").unwrap();
        isolate_settings();
        Fixture { dir }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }
}

/// Where a window opened on `dir` puts trashed items, instead of the real Trash.
pub fn trash_dir(dir: &Path) -> PathBuf {
    let mut name = dir.as_os_str().to_os_string();
    name.push("-trash");
    PathBuf::from(name)
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = std::fs::remove_dir_all(trash_dir(&self.dir));
    }
}

/// Point settings at one empty temp folder, once, before any window opens.
fn isolate_settings() {
    static CONFIG: OnceLock<PathBuf> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let config =
            std::env::temp_dir().join(format!("fffinder-ui-config-{}", std::process::id()));
        // An earlier process with this ID may have left its settings here,
        // and every window of this one would start with them.
        let _ = std::fs::remove_dir_all(&config);
        std::fs::create_dir_all(&config).unwrap();
        // SAFETY: runs once, inside the OnceLock, before any test reads the variable.
        unsafe {
            std::env::set_var("FFFINDER_CONFIG_DIR", &config);
            std::env::set_var("FFFINDER_DATA_DIR", config.join("data"));
        }
        config
    });
}

// ----- hang watchdog -----------------------------------------------------------

fn active_steps() -> &'static Mutex<HashMap<ThreadId, (String, Instant)>> {
    static STEPS: OnceLock<Mutex<HashMap<ThreadId, (String, Instant)>>> = OnceLock::new();
    STEPS.get_or_init(|| {
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let steps = active_steps().lock().unwrap();
                if let Some((step, started)) =
                    steps.values().find(|(_, t)| t.elapsed() > HANG_LIMIT)
                {
                    eprintln!(
                        "UI step `{step}` has run for {:.0?}; the window looks hung",
                        started.elapsed()
                    );
                    std::process::exit(101);
                }
            }
        });
        Mutex::new(HashMap::new())
    })
}

struct StepGuard;

impl StepGuard {
    fn new(label: String) -> StepGuard {
        active_steps()
            .lock()
            .unwrap()
            .insert(std::thread::current().id(), (label, Instant::now()));
        StepGuard
    }
}

impl Drop for StepGuard {
    fn drop(&mut self) {
        active_steps()
            .lock()
            .unwrap()
            .remove(&std::thread::current().id());
    }
}

// ----- the window ----------------------------------------------------------------

pub struct Ui<'a> {
    pub cx: &'a mut TestAppContext,
    pub handle: AnyWindowHandle,
    /// The browser the test drives: the window's first tab, with tabs.
    pub view: Entity<Finder>,
    /// The window's tabs, when opened with them.
    pub workspace: Option<Entity<fffinder::workspace::Workspace>>,
    pub dir: PathBuf,
}

#[derive(Default)]
pub struct Open {
    pub nu: Option<PathBuf>,
    pub picker: Option<PickerOptions>,
    /// Terminals for "Open Nushell Here"; none by default, so tests never
    /// depend on what is installed.
    pub terminals: Vec<Terminal>,
    /// Frames per second in the status bar, as with `--fps`.
    pub fps: bool,
    /// Open the window with tabs and split view, as the app does.
    pub workspace: bool,
    /// Have the index watch the fixture, as the app's does. It then follows
    /// changes on disk in its own time, which a test has to wait for.
    pub watch: bool,
}

impl<'a> Ui<'a> {
    pub fn open(cx: &'a mut TestAppContext, dir: &Path) -> Ui<'a> {
        Ui::open_with(cx, dir, Open::default())
    }

    pub fn open_with(cx: &'a mut TestAppContext, dir: &Path, options: Open) -> Ui<'a> {
        let _guard = StepGuard::new(format!("open {}", dir.display()));
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.to_path_buf()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: options.watch,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(20)));
        let nu = options.nu;
        let terminals = Some(options.terminals);
        cx.update(|cx| {
            fffinder::init(
                cx,
                AppState {
                    engine,
                    nu,
                    terminals,
                },
                Appearance::Light,
            );
            // Tests in this process share a settings folder; stay out of it.
            fffinder::settings::set_persist(false, cx);
            cx.set_global(Recorder::default());
            cx.set_global(fffinder::trash_bin::TrashOverride(std::sync::Arc::new(
                fffinder::trash_bin::FolderBin::new(trash_dir(dir)),
            )));
            if options.fps {
                cx.set_global(fffinder::finder::ShowFps);
            }
            // Dialogs slide in on a wall-clock animation; under load a click
            // could land while one is still moving.
            cx.set_reduce_motion(true);
        });
        let start = Location::Folder(dir.to_path_buf());
        let picker = options.picker;
        let (handle, view, workspace) = if options.workspace {
            let (handle, workspace) = cx
                .update(|cx| {
                    gpui_kit::open_window(WindowOptions::default(), cx, move |window, cx| {
                        let view = cx.new(|cx| {
                            fffinder::workspace::Workspace::new(start, None, picker, window, cx)
                        });
                        let focus = view.read(cx).active().read(cx).files_focus_handle();
                        focus.focus(window, cx);
                        view
                    })
                })
                .unwrap();
            let view = workspace.read_with(cx, |w, _| w.active().clone());
            (handle, view, Some(workspace))
        } else {
            let (handle, view) = cx
                .update(|cx| {
                    gpui_kit::open_window(WindowOptions::default(), cx, move |window, cx| {
                        let view = cx.new(|cx| Finder::new(start, None, picker, window, cx));
                        let focus = view.read(cx).files_focus_handle();
                        focus.focus(window, cx);
                        view
                    })
                })
                .unwrap();
            (handle, view, None)
        };
        let root = dir.to_path_buf();
        view.update(cx, |f, cx| {
            // Settings shows these; the engine already indexes them.
            f.set_search_roots(vec![root], cx);
            // Windows starts the sidebar as a tree, which opens down to the
            // fixture and can push rows like the Trash out of view. Every
            // test starts from the list; tree tests switch to the tree.
            f.set_sidebar_mode(SidebarMode::List, cx);
        });
        let mut ui = Ui {
            cx,
            handle,
            view,
            workspace,
            dir: dir.to_path_buf(),
        };
        ui.settle();
        ui
    }

    /// Let background work and timers run, then draw.
    pub fn settle(&mut self) {
        for _ in 0..6 {
            self.cx.run_until_parked();
            self.cx.executor().advance_clock(Duration::from_millis(100));
        }
        self.cx.run_until_parked();
        self.frame();
    }

    pub fn frame(&mut self) {
        // The window may have closed itself (Cmd-W).
        self.cx
            .update_window(self.handle, |_, window, cx| window.render_frame(cx))
            .ok();
    }

    /// Let the window look for changes on disk `times` over, and let what
    /// each look starts run. It looks every two seconds, so that is how far
    /// the clock goes for each: no further, or a look more could fit in.
    pub fn look_for_changes(&mut self, times: usize) {
        for _ in 0..times {
            self.cx.executor().advance_clock(Duration::from_secs(2));
            self.cx.run_until_parked();
        }
        self.frame();
    }

    /// Rescan the index and wait for it to hold what's on disk now. The
    /// engine counts a rescan as a change to everything.
    pub fn rescan(&mut self) {
        let engine = self.engine();
        self.wait_for_index(&engine);
        engine.rescan();
        assert!(engine.wait_ready(Duration::from_secs(20)));
    }

    /// Build the index anew, so that it holds what's on disk now, without
    /// the engine counting a change. A watcher's index is like that: it has
    /// a file by the time the watcher's word of the file arrives, which
    /// [`Self::report_change`] stands in for.
    pub fn reindex(&mut self) {
        let engine = self.engine();
        engine.set_roots(&[]);
        engine.set_roots(std::slice::from_ref(&self.dir));
        assert!(engine.wait_ready(Duration::from_secs(20)));
    }

    /// Have the engine count a file as the index's watcher reports one that
    /// is gone from `was`, new at `now`, or renamed from one to the other,
    /// and one that is where it was as written to.
    pub fn report_change(&mut self, was: Option<&Path>, now: Option<&Path>) {
        self.engine().report_change_for_test(was, now);
    }

    pub fn engine(&mut self) -> std::sync::Arc<Engine> {
        self.window(|_, cx| cx.global::<AppState>().engine.clone())
    }

    /// Wait until fff-search has finished each index. A rescan asked for
    /// before that is put off, and waiting for it would return at once.
    fn wait_for_index(&mut self, engine: &Engine) {
        let _guard = StepGuard::new("wait for the index".into());
        while engine.status().iter().any(|root| root.finishing) {
            std::thread::yield_now();
        }
    }

    pub fn window<R>(&mut self, f: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        self.cx
            .update_window(self.handle, |_, window, cx| f(window, cx))
            .expect("window is open")
    }

    pub fn read<R>(&self, f: impl FnOnce(&Finder) -> R) -> R {
        self.view.read_with(self.cx, |finder, _| f(finder))
    }

    pub fn update<R>(
        &mut self,
        f: impl FnOnce(&mut Finder, &mut Window, &mut Context<Finder>) -> R,
    ) -> R {
        let view = self.view.clone();
        let result = self.window(|window, cx| view.update(cx, |finder, cx| f(finder, window, cx)));
        self.settle();
        result
    }

    fn step(&mut self, label: String, f: impl FnOnce(&mut Window, &mut App)) {
        let _guard = StepGuard::new(label);
        self.window(f);
        self.settle();
    }

    pub fn click(&mut self, id: impl Into<ElementId>) {
        let id = id.into();
        self.step(format!("click {id:?}"), |window, cx| window.click(id, cx));
    }

    /// Click `id` inside the element `scope`, for IDs that repeat (a
    /// dialog's close button and a notification's share the name `close`).
    pub fn click_in(&mut self, scope: impl Into<ElementId>, id: impl Into<ElementId>) {
        let (scope, id) = (scope.into(), id.into());
        self.step(format!("click {id:?} in {scope:?}"), |window, cx| {
            window.within(scope).click(id, cx)
        });
    }

    /// Click the open dialog's close (×) button.
    pub fn close_dialog(&mut self) {
        self.click_in("dialog", "close");
    }

    /// Drive the Quick Look window instead of the browser's: clicks, keys,
    /// and lookups inside `f` go to it.
    pub fn in_quick_look<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let quick_look = self
            .read(|f| f.quick_look_window())
            .expect("Quick Look is open");
        let browser = std::mem::replace(&mut self.handle, quick_look);
        let result = f(self);
        self.handle = browser;
        self.settle();
        result
    }

    /// Close the Quick Look window, as its close button does.
    pub fn close_quick_look(&mut self) {
        let quick_look = self
            .read(|f| f.quick_look_window())
            .expect("Quick Look is open");
        self.cx
            .update_window(quick_look, |_, window, _| window.remove_window())
            .expect("Quick Look's window is open");
        self.settle();
    }

    /// Close the Quick Look window when it was the last of the two to draw,
    /// and run `then` before the browser's window has drawn again. Until it
    /// has, GPUI has no window on record for the browser, and on macOS a
    /// window isn't drawn while it's minimized or behind another one.
    pub fn close_quick_look_and_then(&mut self, then: impl FnOnce(&mut App)) {
        let _guard = StepGuard::new("close Quick Look".into());
        let quick_look = self
            .read(|f| f.quick_look_window())
            .expect("Quick Look is open");
        let browser = self.view.entity_id();
        // All in one update: the test platform draws every changed window
        // at the end of each.
        self.cx.update(|cx| {
            cx.update_window(quick_look, |_, window, cx| {
                window.render_frame(cx);
                window.remove_window();
            })
            .expect("Quick Look's window is open");
            assert!(
                cx.with_window(browser, |_, _| ()).is_none(),
                "GPUI has no window on record for the browser"
            );
            then(cx);
        });
        self.settle();
    }

    /// Press `key` in the browser's window when the Quick Look window was the
    /// last of the two to draw, as it is whenever the system asks it for a
    /// frame after the browser's. The key comes the way the system sends one:
    /// with no frame drawn first, where `press` draws this window, and with
    /// what it sets off done before the key is let go. It has to follow
    /// another key, since GPUI draws a window before the first key after the
    /// mouse. The key is sent bare, without the text the system types for a
    /// letter, so this is for keys like Return. Spawned work isn't let run
    /// afterwards.
    pub fn press_after_quick_look_draws(&mut self, key: &str) {
        let _guard = StepGuard::new(format!("press {key} after Quick Look draws"));
        let quick_look = self
            .read(|f| f.quick_look_window())
            .expect("Quick Look is open");
        let keystroke = self
            .cx
            .update(|cx| gpui_kit::Keystroke::parse_with_os(key, cx.platform_info().os))
            .unwrap_or_else(|error| panic!("invalid test keystroke: {error}"));
        let down = gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        };
        let up = gpui_kit::KeyUpEvent { keystroke };
        assert!(
            self.window(|window, _| window.last_input_was_keyboard()),
            "a key came before this one"
        );
        self.cx
            .update_window(quick_look, |_, window, cx| window.render_frame(cx))
            .expect("Quick Look's window is open");
        // GPUI answers with the window that last drew the browser when it's
        // asked for the browser's. Were that not Quick Look's here, the key
        // would be like any other, and a test would pass with nothing in
        // its way.
        let browser = self.view.entity_id();
        let drew_last = self
            .cx
            .update(|cx| cx.with_window(browser, |window, _| window.window_handle()));
        assert_eq!(drew_last, Some(quick_look), "Quick Look's window drew last");
        self.window(|window, cx| {
            window.dispatch_event(down.to_platform_input(), cx);
        });
        self.window(|window, cx| {
            window.dispatch_event(up.to_platform_input(), cx);
        });
    }

    /// Click without letting spawned work run afterwards, so a test can
    /// replace that work before it starts.
    pub fn click_without_settling(&mut self, id: impl Into<ElementId>) {
        let id = id.into();
        let _guard = StepGuard::new(format!("click {id:?}"));
        self.window(|window, cx| window.click(id, cx));
    }

    pub fn click_at(&mut self, id: impl Into<ElementId>, offset: Point<Pixels>) {
        let id = id.into();
        self.step(format!("click {id:?} at {offset:?}"), |window, cx| {
            window.click_at(id, offset, cx)
        });
    }

    pub fn double_click(&mut self, id: impl Into<ElementId>) {
        let id = id.into();
        self.step(format!("double-click {id:?}"), |window, cx| {
            window.double_click(id, cx)
        });
    }

    /// Only the second click of a double-click, on the middle of `id`. The
    /// first went to what was at that spot before `id` was, like the button
    /// that opened the dialog `id` is in.
    pub fn second_click(&mut self, id: impl Into<ElementId>) {
        let id = id.into();
        let position = self.snapshot(id.clone()).bounds().center();
        self.step(format!("second click on {id:?}"), |window, cx| {
            click_point_as(window, position, MouseButton::Left, 2, cx)
        });
    }

    /// One click at `position`, on whatever is there, as the `nth` click in
    /// a row at that spot: the second click of a double-click is 2. The
    /// clicks before it have done their work by then.
    pub fn nth_click_point(&mut self, position: Point<Pixels>, nth: usize) {
        self.step(format!("click {nth} at {position:?}"), |window, cx| {
            click_point_as(window, position, MouseButton::Left, nth, cx)
        });
    }

    /// The same with `button`. The system counts the clicks of each button
    /// by themselves.
    pub fn nth_click_point_with(
        &mut self,
        button: MouseButton,
        position: Point<Pixels>,
        nth: usize,
    ) {
        self.step(
            format!("click {nth} with {button:?} at {position:?}"),
            |window, cx| click_point_as(window, position, button, nth, cx),
        );
    }

    pub fn right_click(&mut self, id: impl Into<ElementId>) {
        let id = id.into();
        self.step(format!("right-click {id:?}"), |window, cx| {
            window.right_click(id, cx)
        });
    }

    /// Right-click a point inside `id`, `offset` from its top-left corner.
    pub fn right_click_at(&mut self, id: impl Into<ElementId>, offset: Point<Pixels>) {
        let id = id.into();
        let origin = self.snapshot(id.clone()).bounds().origin;
        self.step(format!("right-click {id:?} at {offset:?}"), |window, cx| {
            click_point(window, origin + offset, MouseButton::Right, cx)
        });
    }

    /// Drag `from` onto `to` with the mouse, holding `modifiers`.
    pub fn drag(
        &mut self,
        from: impl Into<ElementId>,
        to: impl Into<ElementId>,
        modifiers: gpui_kit::Modifiers,
    ) {
        let (from, to) = (from.into(), to.into());
        let start = self.snapshot(from.clone()).bounds().center();
        let end = self.snapshot(to.clone()).bounds().center();
        self.step(format!("drag {from:?} to {to:?}"), |window, cx| {
            let pressed = Some(MouseButton::Left);
            let moves = [
                (start, None),
                (start + point(px(8.), px(8.)), pressed),
                (end, pressed),
            ];
            for (ix, (position, pressed_button)) in moves.into_iter().enumerate() {
                window.render_frame(cx);
                window.dispatch_event(
                    MouseMoveEvent {
                        position,
                        pressed_button,
                        modifiers,
                    }
                    .to_platform_input(),
                    cx,
                );
                if ix == 0 {
                    window.dispatch_event(
                        MouseDownEvent {
                            button: MouseButton::Left,
                            position,
                            modifiers,
                            click_count: 1,
                            first_mouse: false,
                        }
                        .to_platform_input(),
                        cx,
                    );
                }
            }
            window.render_frame(cx);
            window.dispatch_event(
                MouseUpEvent {
                    button: MouseButton::Left,
                    position: end,
                    modifiers,
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
        });
    }

    /// Drop the files at `paths` on `to`, dragged in from another app.
    pub fn drop_from_another_app(&mut self, paths: &[PathBuf], to: impl Into<ElementId>) {
        let to = to.into();
        let position = self.snapshot(to.clone()).bounds().center();
        let paths = gpui_kit::ExternalPaths(paths.iter().cloned().collect());
        self.step(format!("drop files on {to:?}"), |window, cx| {
            // What the system reports as the files come over the window,
            // move across it, and are let go, and when its drag has ended.
            let reports = [
                FileDropEvent::Entered { position, paths },
                FileDropEvent::Pending { position },
                FileDropEvent::Submit { position },
                FileDropEvent::Exited,
            ];
            for report in reports {
                window.render_frame(cx);
                window.dispatch_event(report.to_platform_input(), cx);
            }
            window.render_frame(cx);
        });
    }

    pub fn press(&mut self, key: &str) {
        self.step(format!("press {key}"), |window, cx| window.press(key, cx));
    }

    pub fn type_text(&mut self, text: &str) {
        self.step(format!("type {text:?}"), |window, cx| {
            window.input(text, cx)
        });
    }

    /// Press `key` without letting spawned work run afterwards, as when the
    /// key comes before a search has answered.
    pub fn press_without_settling(&mut self, key: &str) {
        let _guard = StepGuard::new(format!("press {key}"));
        self.window(|window, cx| window.press(key, cx));
    }

    /// Type `text` without letting spawned work run afterwards, so a search
    /// it starts has not answered by the next step.
    pub fn type_without_settling(&mut self, text: &str) {
        let _guard = StepGuard::new(format!("type {text:?}"));
        self.window(|window, cx| window.input(text, cx));
    }

    pub fn dispatch(&mut self, action: impl gpui_kit::Action) {
        let label = format!("dispatch {}", action.name());
        let _guard = StepGuard::new(label);
        self.cx.dispatch_action(self.handle, action);
        self.settle();
    }

    /// Rest the mouse on `id`, long enough for a tooltip to show.
    pub fn hover(&mut self, id: impl Into<ElementId>) {
        let position = self.snapshot(id).bounds().center();
        self.hover_point(position);
    }

    /// Rest the mouse at `position`, long enough for a tooltip to show.
    pub fn hover_point(&mut self, position: Point<Pixels>) {
        self.step(format!("hover {position:?}"), |window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position,
                    pressed_button: None,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
        });
    }

    /// Scroll with the mouse over `id`, by `delta` points.
    pub fn scroll(&mut self, id: impl Into<ElementId>, delta: Point<Pixels>) {
        let position = self.snapshot(id).bounds().center();
        self.scroll_point(position, delta);
    }

    /// Scroll with the mouse at `position`, by `delta` points.
    pub fn scroll_point(&mut self, position: Point<Pixels>, delta: Point<Pixels>) {
        self.step(format!("scroll {position:?}"), |window, cx| {
            window.dispatch_event(
                gpui_kit::ScrollEvent {
                    position,
                    delta: gpui_kit::ScrollDelta::Pixels(delta),
                    modifiers: Default::default(),
                    touch_phase: gpui_kit::TouchPhase::Moved,
                    source: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
        });
    }

    /// Drag `id` sideways by `dx` points.
    pub fn drag_by(&mut self, id: impl Into<ElementId>, dx: f32) {
        let start = self.snapshot(id).bounds().center();
        self.drag_points(start, start + point(px(dx), px(0.)), Default::default());
    }

    /// Press the left button at `start`, drag to `end`, and let go.
    pub fn drag_points(
        &mut self,
        start: Point<Pixels>,
        end: Point<Pixels>,
        modifiers: gpui_kit::Modifiers,
    ) {
        self.step(format!("drag from {start:?} to {end:?}"), |window, cx| {
            press_and_move(window, cx, start, end, modifiers, 1);
            window.dispatch_event(
                MouseUpEvent {
                    button: MouseButton::Left,
                    position: end,
                    modifiers,
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
        });
    }

    /// The same without modifiers, with the press counted as the `nth` click
    /// in a row at `start`: a second click that moves while it is down is 2.
    pub fn nth_drag_points(&mut self, start: Point<Pixels>, end: Point<Pixels>, nth: usize) {
        self.step(
            format!("click {nth} at {start:?}, dragged to {end:?}"),
            |window, cx| {
                press_and_move(window, cx, start, end, Default::default(), nth);
                window.dispatch_event(
                    MouseUpEvent {
                        button: MouseButton::Left,
                        position: end,
                        modifiers: Default::default(),
                        click_count: nth,
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
            },
        );
    }

    /// Press the left button at `start` and drag to `end` without letting go.
    pub fn drag_and_hold(&mut self, start: Point<Pixels>, end: Point<Pixels>) {
        self.drag_and_hold_with(start, end, Default::default());
    }

    /// The same, with `modifiers` held.
    pub fn drag_and_hold_with(
        &mut self,
        start: Point<Pixels>,
        end: Point<Pixels>,
        modifiers: gpui_kit::Modifiers,
    ) {
        self.step(format!("hold from {start:?} to {end:?}"), |window, cx| {
            press_and_move(window, cx, start, end, modifiers, 1);
        });
    }

    /// Move the mouse to `position` with the left button still down.
    pub fn move_while_held(&mut self, position: Point<Pixels>, modifiers: gpui_kit::Modifiers) {
        self.step(format!("hold at {position:?}"), |window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position,
                    pressed_button: Some(MouseButton::Left),
                    modifiers,
                }
                .to_platform_input(),
                cx,
            );
        });
    }

    /// Make the window `width` by `height` points, with a new layout.
    pub fn resize(&mut self, width: f32, height: f32) {
        gpui_kit::VisualTestContext::from_window(self.handle, self.cx)
            .simulate_resize(gpui_kit::size(px(width), px(height)));
        self.settle();
    }

    /// Make the window the active one, as clicking it does. Focus events only
    /// fire in an active window, and a test's window doesn't start out active.
    pub fn activate(&mut self) {
        self.window(|window, _| window.activate_window());
        self.settle();
    }

    /// Make the window inactive, as going to another app does.
    pub fn deactivate(&mut self) {
        gpui_kit::VisualTestContext::from_window(self.handle, self.cx).deactivate_window();
        self.settle();
    }

    /// How many notifications are showing.
    pub fn notification_count(&mut self) -> usize {
        self.window(|window, cx| window.notifications(cx).len())
    }

    /// Put `text` on the clipboard, as copying in another app or a text field does.
    pub fn copy_text(&mut self, text: impl Into<String>) {
        self.cx
            .write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.into()));
    }

    /// Let go of the left button at `position`.
    pub fn release(&mut self, position: Point<Pixels>) {
        self.step(format!("let go at {position:?}"), |window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
        });
    }

    /// Click at `position`, whatever is there.
    pub fn click_point(&mut self, position: Point<Pixels>) {
        self.step(format!("click {position:?}"), |window, cx| {
            click_point(window, position, MouseButton::Left, cx)
        });
    }

    /// Right-click at `position`.
    pub fn right_click_point(&mut self, position: Point<Pixels>) {
        self.step(format!("right-click {position:?}"), |window, cx| {
            click_point(window, position, MouseButton::Right, cx)
        });
    }

    /// Double-click `id` with `modifiers` held.
    pub fn double_click_with(&mut self, id: impl Into<ElementId>, modifiers: gpui_kit::Modifiers) {
        let id = id.into();
        self.step(format!("double-click {id:?}"), |window, cx| {
            let options = gpui_kit::test::ClickOptions::new()
                .with_count(2)
                .with_modifiers(modifiers);
            window.click_with_options(id, options, cx)
        });
    }

    /// Click `button` on the middle of `id`.
    pub fn click_with(&mut self, button: MouseButton, id: impl Into<ElementId>) {
        let id = id.into();
        let position = self.snapshot(id.clone()).bounds().center();
        self.step(format!("click {id:?} with {button:?}"), |window, cx| {
            click_point(window, position, button, cx)
        });
    }

    pub fn snapshot(&mut self, id: impl Into<ElementId>) -> ElementSnapshot {
        let id = id.into();
        self.window(|window, cx| {
            window.render_frame(cx);
            window.find(id)
        })
    }

    pub fn exists(&mut self, id: impl Into<ElementId>) -> bool {
        let id = id.into();
        self.window(|window, cx| {
            window.render_frame(cx);
            window.try_find(id).is_some()
        })
    }

    pub fn menu_open(&mut self) -> bool {
        self.exists("popup-menu")
    }

    pub fn dialog_open(&mut self) -> bool {
        self.window(|window, cx| window.has_active_dialog(cx))
    }

    /// The labels of the open menu's items, top to bottom.
    pub fn menu_labels(&mut self) -> Vec<String> {
        let mut items: Vec<(Pixels, String)> = self
            .snapshots()
            .into_iter()
            .filter(|s| s.role() == Some(gpui_kit::Role::MenuItem))
            .filter_map(|s| Some((s.bounds().origin.y, s.label()?.to_string())))
            .collect();
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        items.into_iter().map(|(_, label)| label).collect()
    }

    /// Click the open menu's item labelled `label`.
    pub fn click_menu_item(&mut self, label: &str) {
        let item = self
            .snapshots()
            .into_iter()
            .find(|s| s.role() == Some(gpui_kit::Role::MenuItem) && s.label() == Some(label))
            .unwrap_or_else(|| panic!("no menu item {label:?}; menu has {:?}", self.menu_labels()));
        assert!(item.visible(), "menu item {label:?} is not visible");
        let center = item.bounds().center();
        self.step(format!("click menu item {label:?}"), |window, cx| {
            click_point(window, center, MouseButton::Left, cx)
        });
    }

    pub fn snapshots(&mut self) -> Vec<ElementSnapshot> {
        self.window(|window, cx| {
            window.render_frame(cx);
            test_support::snapshots(window)
        })
    }

    /// Files handed to the system (open, reveal) so far.
    /// Act as if no app is registered to open `path`.
    pub fn set_no_app(&mut self, path: &Path) {
        self.cx.read(|cx| cx.global::<Recorder>().set_no_app(path));
    }

    pub fn system_calls(&self) -> Vec<SystemCall> {
        self.cx.read(|cx| cx.global::<Recorder>().calls())
    }

    pub fn names(&self) -> Vec<String> {
        self.read(|f| {
            f.rows_for_test()
                .iter()
                .map(|r| r.entry.name.clone())
                .collect()
        })
    }

    pub fn selected(&self) -> Vec<String> {
        self.read(|f| f.selected_entries().into_iter().map(|e| e.name).collect())
    }

    pub fn folder(&self) -> Option<PathBuf> {
        self.read(|f| f.current_folder().map(Path::to_path_buf))
    }

    pub fn files_focused(&mut self) -> bool {
        let view = self.view.clone();
        self.window(|window, cx| view.read(cx).files_focused(window))
    }

    pub fn clipboard(&self) -> String {
        self.cx
            .read_from_clipboard()
            .and_then(|c| c.text())
            .unwrap_or_default()
    }

    /// Navigate to `dir` (inside the fixture) the way the sidebar does.
    pub fn go_to(&mut self, dir: &Path) {
        let dir = dir.to_path_buf();
        self.update(|f, window, cx| f.navigate(Location::Folder(dir), window, cx));
    }

    /// Select the item named `name` with a plain click on its row or tile.
    pub fn select(&mut self, name: &str) {
        let ix = self.index_of(name);
        self.update(|f, window, cx| f.click_row(ix, Default::default(), 1, window, cx));
    }

    /// Add the item named `name` to the selection, as Cmd-click does.
    pub fn select_also(&mut self, name: &str) {
        let ix = self.index_of(name);
        let toggle = if cfg!(target_os = "macos") {
            gpui_kit::Modifiers::command()
        } else {
            gpui_kit::Modifiers::control()
        };
        self.update(|f, window, cx| f.click_row(ix, toggle, 1, window, cx));
    }

    fn index_of(&self, name: &str) -> usize {
        self.names()
            .iter()
            .position(|n| n == name)
            .unwrap_or_else(|| panic!("no item {name:?} in {:?}", self.names()))
    }
}

/// A real mouse click at a window position, through native hit testing.
fn click_point(window: &mut Window, position: Point<Pixels>, button: MouseButton, cx: &mut App) {
    click_point_as(window, position, button, 1, cx)
}

/// The same, counted as the `click_count`th click in a row at that position.
fn click_point_as(
    window: &mut Window,
    position: Point<Pixels>,
    button: MouseButton,
    click_count: usize,
    cx: &mut App,
) {
    window.render_frame(cx);
    window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers: Default::default(),
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        MouseDownEvent {
            button,
            position,
            modifiers: Default::default(),
            click_count,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        MouseUpEvent {
            button,
            position,
            modifiers: Default::default(),
            click_count,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

/// Move to `start`, press the left button as the `click_count`th click in a
/// row there, and drag to `end`, with a frame between each step so the window
/// sees each one as a person would.
fn press_and_move(
    window: &mut Window,
    cx: &mut App,
    start: Point<Pixels>,
    end: Point<Pixels>,
    modifiers: gpui_kit::Modifiers,
    click_count: usize,
) {
    let pressed = Some(MouseButton::Left);
    let moves = [
        (start, None),
        (start + point(px(5.), px(0.)), pressed),
        (end, pressed),
    ];
    for (ix, (position, pressed_button)) in moves.into_iter().enumerate() {
        window.render_frame(cx);
        window.dispatch_event(
            MouseMoveEvent {
                position,
                pressed_button,
                modifiers,
            }
            .to_platform_input(),
            cx,
        );
        if ix == 0 {
            window.dispatch_event(
                MouseDownEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers,
                    click_count,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
        }
    }
    window.render_frame(cx);
}

/// `key` with the platform's command modifier: Cmd on macOS, Ctrl elsewhere.
pub fn cmd(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("cmd-{key}")
    } else {
        format!("ctrl-{key}")
    }
}

/// An offset from an element's top-left corner.
pub fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

pub fn item_id(kind: &'static str, path: &Path) -> ElementId {
    (kind, fffinder::finder::path_id(path)).into()
}
