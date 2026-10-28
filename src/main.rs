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
use wry::{NewWindowResponse, WebViewBuilder};

const HOME_URL: &str = "https://x.com/home";
// x.com serves a degraded page to unknown WebKit user agents, so present as Safari.
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
                          (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
// Hosts that stay inside the app. Everything else opens in the default browser.
const IN_APP_HOSTS: &[&str] = &[
    "x.com",
    "twitter.com",
    "accounts.google.com",
    "appleid.apple.com",
];

actions!(quick_x, [Quit, Back, Forward, Reload, Home, Cut, Copy, Paste, SelectAll]);

enum WebEvent {
    TitleChanged(String),
    OpenUrl(String),
}

struct XApp {
    webview: Entity<WebView>,
    title: SharedString,
}

impl XApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = mpsc::unbounded::<WebEvent>();
        let title_tx = tx.clone();

        let raw = WebViewBuilder::new()
            .with_url(HOME_URL)
            .with_user_agent(USER_AGENT)
            .with_document_title_changed_handler(move |title| {
                let _ = title_tx.unbounded_send(WebEvent::TitleChanged(title));
            })
            // Popups (target=_blank, window.open) have no window to land in, so route them
            // through the app: x.com and sign-in pages load in place, the rest go external.
            .with_new_window_req_handler(move |url, _| {
                let _ = tx.unbounded_send(WebEvent::OpenUrl(url));
                NewWindowResponse::Deny
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
                    WebEvent::OpenUrl(url) if is_in_app(&url) => {
                        this.webview.update(cx, |w, _| w.load_url(&url));
                    }
                    WebEvent::OpenUrl(url) => open_external(&url),
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

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

fn is_in_app(url: &str) -> bool {
    let Some(rest) = url.split_once("://").map(|(_, rest)| rest) else {
        return false;
    };
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    IN_APP_HOSTS
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

fn open_external(url: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(url).spawn();
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
    use super::is_in_app;

    #[test]
    fn keeps_x_and_sign_in_hosts_in_app() {
        assert!(is_in_app("https://x.com/home"));
        assert!(is_in_app("https://mobile.twitter.com/i/flow/login"));
        assert!(is_in_app("https://accounts.google.com/o/oauth2/auth?x=1"));
        assert!(is_in_app("https://X.COM:443/foo"));
    }

    #[test]
    fn sends_other_hosts_external() {
        assert!(!is_in_app("https://t.co/abc"));
        assert!(!is_in_app("https://evilx.com/"));
        assert!(!is_in_app("https://x.com.evil.io/"));
        assert!(!is_in_app("https://x.com@evil.io/"));
        assert!(!is_in_app("not a url"));
    }
}
