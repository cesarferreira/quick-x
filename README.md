# quick-x

A native desktop shell for [x.com](https://x.com), built with [GPUI](https://www.gpui.rs) and the
[gpui-component](https://github.com/longbridge/gpui-component) `WebView` (wry / WKWebView).

## Run

```bash
cargo run --release
```

To install it as `~/Applications/Quick X.app` (so launchers can open it by name):

```bash
scripts/bundle-macos.sh
```

## Features

- GPUI title bar with back, forward, reload and home buttons, plus the page title
- Shortcuts: `⌘[` back, `⌘]` forward, `⌘R` reload, `⌘⇧H` home, `⌘Q` quit
- Native Edit menu, so copy and paste work inside the page
- Login persists between launches (WebKit data store in `~/Library/WebKit/com.cesarferreira.quick-x` for the app bundle, or `~/Library/WebKit/quick-x` with `cargo run`)
- "Sign in with Google/Apple" opens in a separate popup window, like a normal browser. The popup may stay open (blank) after sign-in; close it.
- Other links that open a new window load in the app if they point to x.com or twitter.com; all other web links open in your default browser

## Stack

| Crate | Version |
| --- | --- |
| `gpui` | 0.2.2 |
| `gpui-component` (`webview` feature) | 0.5.1 |
| `lb-wry` | 0.53.3 |

These versions are pinned on purpose: `gpui-component`'s `WebView` only accepts this exact `wry` type.
