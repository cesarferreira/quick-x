#[cfg(target_os = "macos")]
mod popup;

use std::time::Duration;

use futures::{StreamExt as _, channel::mpsc};
use gpui::{
    App, AppContext as _, Application, Bounds, Context, Entity, IntoElement, KeyBinding, Menu,
    MenuItem, OsAction, ParentElement as _, Render, SharedString, Styled as _, Window,
    WindowBounds, WindowOptions, actions, div, px, size,
};
use gpui_component::{
    ActiveTheme as _, Root, Sizable as _, Theme, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    webview::WebView,
};
use url::Url;
use wry::{NewWindowResponse, PageLoadEvent, WebViewBuilder};

const HOME_URL: &str = "https://x.com/home";
// x.com serves a degraded page to unknown WebKit user agents, and Google refuses sign-in
// from embedded browsers, so present as Safari.
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
                          (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
// Links to these hosts load in the main view.
const IN_APP_HOSTS: &[&str] = &["x.com", "twitter.com"];
// Sign-in providers open in a real popup window, so they can report back to x.com.
const AUTH_HOSTS: &[&str] = &["accounts.google.com", "appleid.apple.com"];

actions!(quick_x, [Quit, Back, Forward, Reload, Home, Cut, Copy, Paste, SelectAll]);

enum WebEvent {
    TitleChanged(String),
    LoadInPlace(Url),
}

struct XApp {
    webview: Entity<WebView>,
    title: SharedString,
}

impl XApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = mpsc::unbounded::<WebEvent>();
        let title_tx = tx.clone();

        let debug = std::env::var_os("QUICK_X_DEBUG").is_some();
        let raw = WebViewBuilder::new()
            .with_url(HOME_URL)
            .with_user_agent(USER_AGENT)
            .with_document_title_changed_handler(move |title| {
                let _ = title_tx.unbounded_send(WebEvent::TitleChanged(title));
            })
            // Popups (target=_blank, window.open): sign-in gets a real popup window, x.com
            // links load in place, other web links open in the default browser.
            .with_on_page_load_handler(move |event, url| {
                if debug {
                    let state = match event {
                        PageLoadEvent::Started => "started",
                        PageLoadEvent::Finished => "finished",
                    };
                    eprintln!("[quick-x] main {state} {url}");
                }
                // Sign-in finished and x.com moved on: the popup has done its job.
                #[cfg(target_os = "macos")]
                if matches!(event, PageLoadEvent::Finished) && is_signed_in_page(&url) {
                    popup::close_all();
                }
            })
            .with_new_window_req_handler(move |url, features| {
                if debug {
                    eprintln!("[quick-x] new window {url}");
                }
                match web_url(&url) {
                    #[cfg(target_os = "macos")]
                    Some(url) if matches_host(&url, AUTH_HOSTS) => popup::open(features, USER_AGENT),
                    Some(url) if matches_host(&url, IN_APP_HOSTS) || matches_host(&url, AUTH_HOSTS) => {
                        let _ = tx.unbounded_send(WebEvent::LoadInPlace(url));
                        NewWindowResponse::Deny
                    }
                    Some(url) => {
                        open_external(&url);
                        NewWindowResponse::Deny
                    }
                    None => NewWindowResponse::Deny,
                }
            })
            .build_as_child(window)
            .expect("failed to create webview");

        let webview = cx.new(|cx| WebView::new(raw, window, cx));

        cx.spawn(async move |this, cx| {
            while let Some(event) = rx.next().await {
                let result = this.update(cx, |this, cx| match event {
                    WebEvent::TitleChanged(title) => {
                        this.title = title.into();
                        cx.notify();
                    }
                    WebEvent::LoadInPlace(url) => {
                        this.webview.update(cx, |w, _| w.load_url(url.as_str()));
                    }
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        #[cfg(target_os = "macos")]
        if debug {
            cx.spawn(async move |_, cx| {
                let mut last = Vec::new();
                loop {
                    cx.background_executor().timer(Duration::from_millis(500)).await;
                    let now = popup::describe();
                    if now != last {
                        now.iter().for_each(|line| eprintln!("[quick-x] {line}"));
                        last = now;
                    }
                }
            })
            .detach();
        }

        Self {
            webview,
            title: "X".into(),
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.webview.update(cx, |w, _| _ = w.back());
    }

    fn forward(&mut self, cx: &mut Context<Self>) {
        self.webview
            .update(cx, |w, _| _ = w.evaluate_script("history.forward();"));
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.webview.update(cx, |w, _| _ = w.reload());
    }

    fn home(&mut self, cx: &mut Context<Self>) {
        self.webview.update(cx, |w, _| w.load_url(HOME_URL));
    }
}

impl Render for XApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title_bar = TitleBar::new().child(
            h_flex()
                .w_full()
                .pr_2()
                .gap_1()
                .child(
                    Button::new("back")
                        .ghost()
                        .small()
                        .label("←")
                        .tooltip("Back")
                        .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                )
                .child(
                    Button::new("forward")
                        .ghost()
                        .small()
                        .label("→")
                        .tooltip("Forward")
                        .on_click(cx.listener(|this, _, _, cx| this.forward(cx))),
                )
                .child(
                    Button::new("reload")
                        .ghost()
                        .small()
                        .label("↻")
                        .tooltip("Reload")
                        .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                )
                .child(
                    Button::new("home")
                        .ghost()
                        .small()
                        .label("⌂")
                        .tooltip("Home")
                        .on_click(cx.listener(|this, _, _, cx| this.home(cx))),
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(self.title.clone()),
                ),
        );

        window.set_window_title(&self.title);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(title_bar)
            .child(div().flex_1().child(self.webview.clone()))
    }
}

/// Parses `url` and keeps it only if it is a web URL. Other schemes (file, javascript,
/// custom app handlers) must never be loaded or handed to the OS opener.
fn web_url(url: &str) -> Option<Url> {
    Url::parse(url)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
}

fn matches_host(url: &Url, hosts: &[&str]) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    hosts
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

/// True once x.com shows a page that only signed-in users get.
fn is_signed_in_page(url: &str) -> bool {
    web_url(url).is_some_and(|url| {
        matches_host(&url, IN_APP_HOSTS) && !url.path().starts_with("/i/flow/") && url.path() != "/"
    })
}

fn open_external(url: &Url) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(url.as_str()).spawn();
}

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        Theme::sync_system_appearance(None, cx);

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-[", Back, None),
            KeyBinding::new("cmd-]", Forward, None),
            KeyBinding::new("cmd-r", Reload, None),
            KeyBinding::new("cmd-shift-h", Home, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-a", SelectAll, None),
        ]);
        // Edit items use native selectors so copy/paste reach the focused web content.
        cx.set_menus(vec![
            Menu {
                name: "X".into(),
                items: vec![MenuItem::action("Quit X", Quit)],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Cut", Cut, OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, OsAction::Paste),
                    MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                ],
            },
            Menu {
                name: "View".into(),
                items: vec![
                    MenuItem::action("Back", Back),
                    MenuItem::action("Forward", Forward),
                    MenuItem::action("Reload", Reload),
                    MenuItem::separator(),
                    MenuItem::action("Home", Home),
                ],
            },
        ]);

        let bounds = Bounds::centered(None, size(px(600.), px(900.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(size(px(380.), px(500.))),
            ..Default::default()
        };

        let mut app = None;
        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| XApp::new(window, cx));
            app = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open window");

        // The native webview owns keyboard focus, so shortcuts arrive as menu key
        // equivalents with no focused GPUI element. Handle them app-wide.
        let app = app.expect("window view");
        let a = app.clone();
        cx.on_action(move |_: &Back, cx| a.update(cx, |this, cx| this.back(cx)));
        let a = app.clone();
        cx.on_action(move |_: &Forward, cx| a.update(cx, |this, cx| this.forward(cx)));
        let a = app.clone();
        cx.on_action(move |_: &Reload, cx| a.update(cx, |this, cx| this.reload(cx)));
        cx.on_action(move |_: &Home, cx| app.update(cx, |this, cx| this.home(cx)));
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::{AUTH_HOSTS, IN_APP_HOSTS, matches_host, web_url};

    fn matches(url: &str, hosts: &[&str]) -> bool {
        web_url(url).is_some_and(|url| matches_host(&url, hosts))
    }

    #[test]
    fn keeps_x_links_in_app() {
        assert!(matches("https://x.com/home", IN_APP_HOSTS));
        assert!(matches("https://mobile.twitter.com/i/flow/login", IN_APP_HOSTS));
        assert!(matches("https://X.COM:443/foo", IN_APP_HOSTS));
        assert!(!matches("https://accounts.google.com/o/oauth2/auth", IN_APP_HOSTS));
    }

    #[test]
    fn recognises_sign_in_providers() {
        assert!(matches("https://accounts.google.com/o/oauth2/auth?x=1", AUTH_HOSTS));
        assert!(matches("https://appleid.apple.com/auth/authorize", AUTH_HOSTS));
        assert!(!matches("https://x.com/home", AUTH_HOSTS));
    }

    #[test]
    fn sends_other_hosts_external() {
        for url in [
            "https://t.co/abc",
            "https://evilx.com/",
            "https://x.com.evil.io/",
            "https://x.com@evil.io/",
            "https://evil.io/accounts.google.com",
        ] {
            assert!(!matches(url, IN_APP_HOSTS), "{url}");
            assert!(!matches(url, AUTH_HOSTS), "{url}");
        }
        assert!(web_url("https://t.co/abc").is_some());
    }

    #[test]
    fn rejects_non_web_schemes() {
        assert!(web_url("file:///Applications/Calculator.app").is_none());
        assert!(web_url("javascript://x.com/%0aalert(1)").is_none());
        assert!(web_url("smb://x.com/share").is_none());
        assert!(web_url("vscode://file/etc/passwd").is_none());
        assert!(web_url("not a url").is_none());
    }
}
