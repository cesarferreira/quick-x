# quick-x

A native desktop shell for [x.com](https://x.com), built with [GPUI](https://www.gpui.rs) and the
[gpui-component](https://github.com/longbridge/gpui-component) `WebView` (wry / WKWebView).

## Run

```bash
cargo run --release
```

## Features

- GPUI title bar with back, forward, reload and home buttons, plus the page title
- Shortcuts: `⌘[` back, `⌘]` forward, `⌘R` reload, `⌘⇧H` home, `⌘Q` quit
- Native Edit menu, so copy and paste work inside the page
- Login persists between launches (WebKit data store in `~/Library/WebKit/quick-x`)
- Links that open a new window load in the app if they point to x.com, twitter.com or the Google/Apple sign-in pages; all other links open in your default browser
- Sign in with a username and password. "Sign in with Google/Apple" is untested and probably does not work in an embedded webview.

## Stack

| Crate | Version |
| --- | --- |
| `gpui` | 0.2.2 |
| `gpui-component` (`webview` feature) | 0.5.1 |
| `lb-wry` | 0.53.3 |

These versions are pinned on purpose: `gpui-component`'s `WebView` only accepts this exact `wry` type.
