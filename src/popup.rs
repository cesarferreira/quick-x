//! Native popup windows for sign-in providers (macOS only).
//!
//! wry can open popups itself, but then the app has no handle on them: the popup keeps
//! WebKit's default (embedded-browser) user agent and never closes. Owning the window
//! lets the popup present as Safari, close once sign-in lands, and be inspected.

use std::cell::RefCell;

use objc2::{MainThreadMarker, MainThreadOnly as _, rc::Retained};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_web_kit::WKWebView;
use wry::{NewWindowFeatures, NewWindowResponse};

struct Popup {
    window: Retained<NSWindow>,
    webview: Retained<WKWebView>,
}

thread_local! {
    static POPUPS: RefCell<Vec<Popup>> = const { RefCell::new(Vec::new()) };
}

/// Opens a popup window for a `window.open` request, keeping the opener relationship.
pub fn open(features: NewWindowFeatures, user_agent: &str) -> NewWindowResponse {
    let Some(mtm) = MainThreadMarker::new() else {
        return NewWindowResponse::Deny;
    };

    let size = features
        .size
        .map_or(NSSize::new(500., 650.), |s| NSSize::new(s.width, s.height));
    let rect = NSRect::new(NSPoint::new(0., 0.), size);
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;

    // SAFETY: called on the main thread (checked above) with a configuration WebKit gave us
    // for this request, which is what `webView:createWebViewWithConfiguration:` requires.
    let (window, webview) = unsafe {
        let window = NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        // We own the window through `Retained`, so AppKit must not release it on close.
        window.setReleasedWhenClosed(false);

        let webview = WKWebView::initWithFrame_configuration(
            WKWebView::alloc(mtm),
            rect,
            &features.opener.target_configuration,
        );
        webview.setCustomUserAgent(Some(&NSString::from_str(user_agent)));
        webview.setInspectable(true);
        (window, webview)
    };

    window.setContentView(Some(&webview));
    window.center();
    window.makeKeyAndOrderFront(None);

    POPUPS.with_borrow_mut(|popups| {
        popups.retain(|p| p.window.isVisible());
        popups.push(Popup {
            window,
            webview: webview.clone(),
        });
    });
    NewWindowResponse::Create { webview }
}

/// Closes every open popup. Sign-in pages call `window.close()`, which WebKit ignores
/// for popups without a UI delegate, so the app closes them once login completes.
pub fn close_all() {
    POPUPS.with_borrow_mut(|popups| {
        for popup in popups.drain(..) {
            popup.window.close();
        }
    });
}

/// One line per visible popup: loading state, URL and title. Used for debug logging.
pub fn describe() -> Vec<String> {
    POPUPS.with_borrow(|popups| {
        popups
            .iter()
            .filter(|p| p.window.isVisible())
            .map(|p| {
                // SAFETY: plain property reads on the main thread.
                let (loading, url, title) = unsafe {
                    (
                        p.webview.isLoading(),
                        p.webview
                            .URL()
                            .and_then(|u| u.absoluteString())
                            .map(|s| s.to_string()),
                        p.webview.title().map(|s| s.to_string()),
                    )
                };
                format!(
                    "popup loading={loading} url={} title={:?}",
                    url.unwrap_or_default(),
                    title.unwrap_or_default()
                )
            })
            .collect()
    })
}
