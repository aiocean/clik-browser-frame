//! `browser-frame`: macOS-style window chrome (outline, title bar, traffic lights, URL pill)
//! with a transparent body, to sit over a screenshot.

use clik_plugin::v1;

#[cfg(target_family = "wasm")]
mod guest {
    use clik_plugin::{Output, Plugin, v1};

    struct BrowserFrame;

    impl Plugin for BrowserFrame {
        fn run(step: &str, _args: &[String], _input: v1::Input) -> Result<Output, String> {
            Err(format!("unknown step: {step}"))
        }

        fn render(kind: &str, input: v1::RenderInput) -> Result<v1::RenderOutput, String> {
            match kind {
                "browser-frame" => Ok(super::browser_frame(&input)),
                other => Err(format!("unknown shape kind: {other}")),
            }
        }
    }

    clik_plugin::export!(BrowserFrame);
}

/// clik lays plugin text out in a box `size * LINE_HEIGHT` tall from `y`
/// (`canvas-core::bounds::LINE_HEIGHT`).
const LINE_HEIGHT: f32 = 1.35;
const BAR_HEIGHT: f32 = 28.0;
const DOT_RADIUS: f32 = 6.0;
/// Center of the first dot and the distance between dot centers.
const DOT_START: f32 = 18.0;
const DOT_STEP: f32 = 20.0;
const DOTS: [u32; 3] = [0xFF5F57FF, 0xFEBC2EFF, 0x28C840FF];
const URL_SIZE: f32 = 12.0;
const PILL_HEIGHT: f32 = 18.0;
/// Space kept clear of the dots on both sides, so the pill stays centered on the window.
const PILL_SIDE: f32 = 76.0;
const PILL_MIN_WIDTH: f32 = 48.0;

struct Theme {
    bar: u32,
    outline: u32,
    separator: u32,
    pill: u32,
    url: u32,
    dot_edge: u32,
}

const LIGHT: Theme = Theme {
    bar: 0xEBEBEDFF,
    outline: 0xC4C4C8FF,
    separator: 0xD1D1D6FF,
    pill: 0xFFFFFFFF,
    url: 0x5A5A5FFF,
    dot_edge: 0x0000001F,
};

const DARK: Theme = Theme {
    bar: 0x2C2C2EFF,
    outline: 0x4A4A4EFF,
    separator: 0x1C1C1EFF,
    pill: 0x1C1C1EFF,
    url: 0xAEAEB2FF,
    dot_edge: 0x00000040,
};

fn browser_frame(input: &v1::RenderInput) -> v1::RenderOutput {
    let (w, h) = (input.width.max(1.0), input.height.max(1.0));
    let theme = match input.props.get("theme") {
        Some(v1::PropValue::Enum(t)) if t == "dark" => &DARK,
        _ => &LIGHT,
    };
    let url = match input.props.get("url") {
        // Shown like a browser's address bar: without the scheme or a trailing slash.
        Some(v1::PropValue::Text(t)) => {
            let t = t.strip_prefix("https://").or_else(|| t.strip_prefix("http://")).unwrap_or(t);
            t.strip_suffix('/').unwrap_or(t)
        }
        _ => "example.com",
    };
    let show_url = !matches!(
        input.props.get("show_url"),
        Some(v1::PropValue::Bool(false))
    );

    // Strokes sit half a pixel in, so the 1 px outline stays inside the box.
    let half = 0.5;
    let radius = 10.0_f32.min(w / 4.0).min(h / 4.0);
    let bar_h = BAR_HEIGHT.min(h - half);
    let mut primitives = vec![
        v1::Primitive::Path {
            segs: top_rounded(half, half, w - half, bar_h, radius),
            fill: Some(theme.bar),
            stroke: None,
        },
        v1::Primitive::Path {
            segs: vec![v1::Seg::Move(half, bar_h), v1::Seg::Line(w - half, bar_h)],
            fill: None,
            stroke: Some(stroke(theme.separator)),
        },
    ];

    let cy = bar_h / 2.0;
    if bar_h >= 2.0 * DOT_RADIUS + 4.0 {
        for (i, color) in DOTS.into_iter().enumerate() {
            let cx = DOT_START + DOT_STEP * i as f32;
            if cx + DOT_RADIUS > w - radius {
                break;
            }
            primitives.push(v1::Primitive::Path {
                segs: circle(cx, cy, DOT_RADIUS - 0.25),
                fill: Some(color),
                stroke: Some(v1::StrokeSpec {
                    width: 0.5,
                    color: theme.dot_edge,
                    dash: None,
                }),
            });
        }
    }

    let pill_w = (w - 2.0 * PILL_SIDE).min(480.0);
    if show_url && !url.is_empty() && pill_w >= PILL_MIN_WIDTH && bar_h >= PILL_HEIGHT + 4.0 {
        let pill_x = (w - pill_w) / 2.0;
        let pill_y = cy - PILL_HEIGHT / 2.0;
        primitives.push(v1::Primitive::Path {
            segs: rounded_rect(pill_x, pill_y, pill_x + pill_w, pill_y + PILL_HEIGHT, 6.0),
            fill: Some(theme.pill),
            stroke: None,
        });
        let pad = 8.0;
        let room = pill_w - 2.0 * pad;
        let text = fit(url, room / URL_SIZE);
        let text_w = estimate_em(&text) * URL_SIZE;
        // clik aligns plugin text to the start of its box: center from the estimated width,
        // and run the box to the pill's right padding so an underestimate never wraps.
        let x = (pill_x + (pill_w - text_w) / 2.0).max(pill_x + pad);
        primitives.push(v1::Primitive::Text {
            x,
            y: cy - URL_SIZE * LINE_HEIGHT / 2.0,
            width: pill_x + pill_w - pad - x,
            text,
            font: "sans".into(),
            size: URL_SIZE,
            color: theme.url,
        });
    }

    // Outline last, over the bar edge.
    primitives.push(v1::Primitive::Path {
        segs: rounded_rect(half, half, w - half, h - half, radius),
        fill: None,
        stroke: Some(stroke(theme.outline)),
    });
    v1::RenderOutput { primitives }
}

fn stroke(color: u32) -> v1::StrokeSpec {
    v1::StrokeSpec {
        width: 1.0,
        color,
        dash: None,
    }
}

/// `text`, cut with an ellipsis to fit `max_em` (estimated).
fn fit(text: &str, max_em: f32) -> String {
    if estimate_em(text) <= max_em {
        return text.to_string();
    }
    let mut out = String::new();
    let mut em = estimate_em("…");
    for c in text.chars() {
        let next = estimate_em(c.encode_utf8(&mut [0; 4]));
        if em + next > max_em {
            break;
        }
        em += next;
        out.push(c);
    }
    out.push('…');
    out
}

/// Width of `text` in em for the system sans font at small sizes; close enough to center a URL.
fn estimate_em(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            ' ' => 0.28,
            'i' | 'j' | 'l' | 'I' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | '/' => 0.3,
            'f' | 't' | 'r' => 0.4,
            'm' | 'w' | 'M' | 'W' => 0.85,
            'A'..='Z' => 0.66,
            'a'..='z' | '0'..='9' => 0.57,
            c if c.is_ascii() => 0.6,
            _ => 0.9,
        })
        .sum()
}

/// A rectangle from (x0, y0) to (x1, y1) with only its top corners rounded.
fn top_rounded(x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) -> Vec<v1::Seg> {
    let r = radius.min((x1 - x0) / 2.0).min(y1 - y0).max(0.0);
    let k = r * 0.552_284_8;
    vec![
        v1::Seg::Move(x0, y1),
        v1::Seg::Line(x0, y0 + r),
        v1::Seg::Cubic(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0),
        v1::Seg::Line(x1 - r, y0),
        v1::Seg::Cubic(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r),
        v1::Seg::Line(x1, y1),
        v1::Seg::Close,
    ]
}

/// A closed rounded rectangle; the radius is clamped to half the shorter side.
fn rounded_rect(x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) -> Vec<v1::Seg> {
    let r = radius.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    let k = r * 0.552_284_8;
    vec![
        v1::Seg::Move(x0 + r, y0),
        v1::Seg::Line(x1 - r, y0),
        v1::Seg::Cubic(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r),
        v1::Seg::Line(x1, y1 - r),
        v1::Seg::Cubic(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1),
        v1::Seg::Line(x0 + r, y1),
        v1::Seg::Cubic(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r),
        v1::Seg::Line(x0, y0 + r),
        v1::Seg::Cubic(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0),
        v1::Seg::Close,
    ]
}

fn circle(cx: f32, cy: f32, r: f32) -> Vec<v1::Seg> {
    rounded_rect(cx - r, cy - r, cx + r, cy + r, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clik_plugin::testing;
    use std::collections::BTreeMap;

    fn render(w: f32, h: f32, props: &[(&str, v1::PropValue)]) -> v1::RenderOutput {
        let props: BTreeMap<_, _> = props
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        browser_frame(&testing::render_input(w, h, props))
    }

    fn text(out: &v1::RenderOutput) -> Option<(f32, f32, f32, String)> {
        out.primitives.iter().find_map(|p| match p {
            v1::Primitive::Text {
                x, y, width, text, ..
            } => Some((*x, *y, *width, text.clone())),
            _ => None,
        })
    }

    fn dot_count(out: &v1::RenderOutput) -> usize {
        out.primitives
            .iter()
            .filter(|p| matches!(p, v1::Primitive::Path { fill: Some(c), .. } if DOTS.contains(c)))
            .count()
    }

    #[test]
    fn body_is_transparent() {
        let out = render(400.0, 300.0, &[]);
        // Only the bar, the dots and the pill are filled; nothing below the bar.
        for p in &out.primitives {
            if let v1::Primitive::Path {
                segs,
                fill: Some(_),
                ..
            } = p
            {
                let below = segs.iter().any(|s| match s {
                    v1::Seg::Move(_, y) | v1::Seg::Line(_, y) => *y > BAR_HEIGHT + 0.01,
                    _ => false,
                });
                assert!(!below, "a fill reaches into the body");
            }
        }
    }

    #[test]
    fn draws_three_dots_and_a_centered_url() {
        let out = render(400.0, 300.0, &[]);
        assert_eq!(dot_count(&out), 3);
        let (x, y, width, t) = text(&out).unwrap_or_default();
        assert_eq!(t, "example.com");
        let center = x + estimate_em(&t) * URL_SIZE / 2.0;
        assert!((center - 200.0).abs() < 0.5, "center {center}");
        assert!(y >= 0.0 && y + URL_SIZE * LINE_HEIGHT <= BAR_HEIGHT);
        assert!(estimate_em(&t) * URL_SIZE <= width);
    }

    #[test]
    fn show_url_false_hides_the_pill() {
        let on = render(400.0, 300.0, &[]);
        let off = render(400.0, 300.0, &[("show_url", v1::PropValue::Bool(false))]);
        assert!(text(&off).is_none());
        assert_eq!(off.primitives.len() + 2, on.primitives.len());
    }

    #[test]
    fn long_url_is_cut_to_fit() {
        let long = "very-long-subdomain.example.com/a/really/long/path/that/goes/on";
        let out = render(300.0, 80.0, &[("url", v1::PropValue::Text(long.into()))]);
        let (_, _, width, t) = text(&out).unwrap_or_default();
        assert!(t.ends_with('…'));
        assert!(estimate_em(&t) * URL_SIZE <= width + 0.01);
    }

    #[test]
    fn narrow_frame_drops_what_does_not_fit() {
        let out = render(120.0, 40.0, &[]);
        assert_eq!(dot_count(&out), 3);
        assert!(text(&out).is_none(), "no room for a pill at 120 px");
        let tiny = render(30.0, 10.0, &[]);
        assert_eq!(dot_count(&tiny), 0);
    }
}
