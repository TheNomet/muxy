#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::*;
use crate::browser::Options;
use crate::webview::linux::browser::NativeBrowser;
use gpui::Keystroke;
use polybrowser::Profile;

const WAIT: Duration = Duration::from_secs(20);

#[derive(Debug)]
enum Input {
    Pointer(c_int, f64, f64),
    Key(u32),
    Scroll,
}

fn with_page<T: Send + 'static>(job: impl FnOnce(&Runtime, &Page) -> T + Send + 'static) -> T {
    let (send, receive) = mpsc::channel();
    post(move |runtime| {
        let runtime = runtime.unwrap();
        let page = runtime.pages.borrow().values().next().unwrap().clone();
        send.send(job(runtime, &page)).ok();
    });
    receive.recv_timeout(WAIT).unwrap()
}

unsafe extern "C" fn on_input(_widget: gpointer, event: gpointer, data: gpointer) -> gboolean {
    let sender = unsafe { &*data.cast::<mpsc::Sender<Input>>() };
    let kind = unsafe { (*event.cast::<GdkEventAny>()).kind };
    let input = match kind {
        ffi::EVENT_BUTTON_PRESS | ffi::EVENT_BUTTON_RELEASE => {
            let event = unsafe { &*event.cast::<GdkEventButton>() };
            Input::Pointer(kind, event.x, event.y)
        }
        ffi::EVENT_KEY_PRESS => Input::Key(unsafe { (*event.cast::<GdkEventKey>()).keyval }),
        ffi::EVENT_SCROLL => Input::Scroll,
        _ => return FALSE,
    };
    let _ = sender.send(input);
    FALSE
}

unsafe extern "C" fn free_sender(data: gpointer, _closure: gpointer) {
    drop(unsafe { Box::from_raw(data.cast::<mpsc::Sender<Input>>()) });
}

fn attach_inspector() -> (mpsc::Receiver<Input>, (f64, f64)) {
    with_page(|runtime, page| unsafe {
        let library = libloading::Library::new("libwebkit2gtk-4.1.so.0").unwrap();
        let attach = library
            .get::<unsafe extern "C" fn(gpointer)>(b"webkit_web_inspector_attach\0")
            .unwrap();
        attach(runtime.g.webkit_web_view_get_inspector(page.view));
    });
    let deadline = Instant::now() + WAIT;
    let position = loop {
        let position = with_page(|runtime, page| unsafe {
            let g = runtime.g;
            let inspector = g.webkit_web_view_get_inspector(page.view);
            let view = g.webkit_web_inspector_get_web_view(inspector);
            if view.is_null() || g.webkit_web_inspector_is_attached(inspector) == FALSE {
                return None;
            }
            let (mut x, mut y) = (0, 0);
            let width = g.gtk_widget_get_allocated_width(view);
            let height = g.gtk_widget_get_allocated_height(view);
            (width > 1
                && height > 1
                && g.gtk_widget_translate_coordinates(
                    view, page.view, 0, 0, &raw mut x, &raw mut y,
                ) != FALSE)
                .then_some((f64::from(x + width / 2), f64::from(y + height / 2)))
        });
        if let Some(position) = position {
            break position;
        }
        assert!(Instant::now() < deadline, "inspector did not attach");
        std::thread::sleep(Duration::from_millis(10));
    };
    let (send, receive) = mpsc::channel();
    with_page(move |runtime, page| unsafe {
        let g = runtime.g;
        let view = g.webkit_web_inspector_get_web_view(g.webkit_web_view_get_inspector(page.view));
        g.g_signal_connect_data(
            view,
            c"event".as_ptr(),
            callback!(on_input(gpointer, gpointer, gpointer) -> gboolean),
            Box::into_raw(Box::new(send)).cast(),
            free_sender as *const () as gpointer,
            0,
        );
    });
    (receive, position)
}

fn eval(browser: &NativeBrowser, script: &str) -> String {
    let (send, receive) = mpsc::channel();
    browser
        .run_script(script, move |value| {
            let _ = send.send(value);
        })
        .unwrap();
    receive.recv_timeout(WAIT).unwrap().unwrap()
}

fn open_browser() -> NativeBrowser {
    let (browser, events) = NativeBrowser::open(&Options {
        profile: &Profile::ephemeral(),
        url: Some("about:blank"),
        allows: |_| true,
    })
    .unwrap();
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(1000.0), px(800.0)));
    browser.sync(bounds, bounds, 0.0, 2.0);
    browser.set_visible(true);
    browser.set_focused(true);
    let deadline = Instant::now() + WAIT;
    loop {
        match events.try_recv() {
            Ok(crate::browser::Event::Page {
                event: polybrowser::BrowserEvent::LoadFinished { .. },
                ..
            }) => return browser,
            Ok(crate::browser::Event::Page {
                event: polybrowser::BrowserEvent::LoadFailed { error, .. },
                ..
            }) => panic!("{error}"),
            _ => {
                assert!(Instant::now() < deadline, "page did not load");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

#[test]
#[ignore = "Requires WebKitGTK and a display; runs in the Linux desktop guard"]
fn docked_inspector_receives_pointer_and_keyboard_input() {
    let _serial = crate::webview::linux::tests::alone();
    let browser = open_browser();
    eval(
        &browser,
        "window.clicks = 0; window.keys = 0; addEventListener('mousedown', () => clicks++); addEventListener('keydown', () => keys++); return true;",
    );
    browser.open_devtools().unwrap();
    let (input, (x, y)) = attach_inspector();
    #[allow(clippy::cast_possible_truncation)]
    let position = point(px((x / 2.0) as f32), px((y / 2.0) as f32));
    browser.pointer(Pointer::Pressed {
        position,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
    });
    let Input::Pointer(kind, local_x, local_y) = input.recv_timeout(WAIT).unwrap() else {
        panic!("inspector did not receive the click")
    };
    assert_eq!(kind, ffi::EVENT_BUTTON_PRESS);
    assert!(local_x >= 0.0 && local_y >= 0.0 && (local_x < x || local_y < y));
    browser.pointer(Pointer::Released {
        position: point(px(2.0), px(2.0)),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
    });
    assert!(matches!(
        input.recv_timeout(WAIT).unwrap(),
        Input::Pointer(ffi::EVENT_BUTTON_RELEASE, ..)
    ));
    browser.pointer(Pointer::Scrolled {
        position,
        delta: point(px(0.0), px(-30.0)),
        modifiers: Modifiers::default(),
    });
    assert!(matches!(input.recv_timeout(WAIT).unwrap(), Input::Scroll));
    browser.key(&Keystroke::parse("x").unwrap(), true, || None);
    assert!(matches!(input.recv_timeout(WAIT).unwrap(), Input::Key(120)));
    assert_eq!(eval(&browser, "return [clicks, keys]"), "[0,0]");
    browser.close_devtools().unwrap();
    browser.pointer(Pointer::Pressed {
        position: point(px(2.0), px(2.0)),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
    });
    browser.pointer(Pointer::Released {
        position: point(px(2.0), px(2.0)),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
    });
    let deadline = Instant::now() + WAIT;
    while eval(&browser, "return clicks") != "1" {
        assert!(
            Instant::now() < deadline,
            "page did not receive input after inspector closed"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(browser);
    let (send, receive) = mpsc::channel();
    post(move |runtime| send.send(runtime.unwrap().live()).unwrap());
    assert_eq!(receive.recv_timeout(WAIT).unwrap(), 0);
}
