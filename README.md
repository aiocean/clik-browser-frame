# clik-browser-frame

A [clik](https://clik.aiocean.io) plugin that adds macOS-style window chrome as a shape: a rounded outline, a 28 px title bar with red, yellow and green dots, and an optional URL pill. The body is transparent, so the frame sits over a screenshot. A URL too long for the pill is cut with an ellipsis; on a narrow frame the pill is left out.

## Steps

| Step | Role | What it does |
|---|---|---|
| `browser-frame` | shape | Draws browser window chrome |

## Props

| Prop | Kind | Default | Meaning |
|---|---|---|---|
| `url` | text | `example.com` | Text in the URL pill |
| `theme` | `light` \| `dark` | `light` | Chrome colors |
| `show_url` | bool | `true` | Show the URL pill |

To put a screenshot in the frame, right-click the screenshot and choose **Frame with → Browser frame**: the frame is sized so the screenshot fills its body, and both are grouped.

No config, no network access.

## Install

In clik, open Settings → Plugins and either find it under **Browse**, or choose **Install from GitHub** and enter `aiocean/clik-browser-frame`.

## Build

Needs the `wasm32-unknown-unknown` target and `wasm-tools`.

```bash
cargo test
cargo build --release --target wasm32-unknown-unknown
mkdir -p pkg
wasm-tools component new target/wasm32-unknown-unknown/release/clik_browser_frame.wasm -o pkg/plugin.wasm
cp plugin.toml pkg/
```

Pushing a `v*` tag matching `version` in `plugin.toml` publishes `browser-frame-<version>.clikplugin.tar.gz` as a GitHub release (`.github/workflows/release.yml`).

## License

MIT
