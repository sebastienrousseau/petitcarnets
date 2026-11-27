use crate::color::{ColorProfile, Rgb};
use crate::key::Key;
use crate::mode::{KeyboardEnhancementsReport, ModeSetting, ReportedMode};
use crate::mouse::{Mouse, MouseButton};
use crate::msg::{ClipboardSelection, ColorScheme, Msg, Size};

/// One of every runtime message.
fn every_runtime_message() -> Vec<Msg<u8>> {
    let mouse = Mouse {
        x: 1,
        y: 2,
        button: MouseButton::Left,
        mods: crate::KeyMod::NONE,
    };
    vec![
        Msg::KeyPress(Key::char('a')),
        Msg::KeyRelease(Key::char('b')),
        Msg::MouseClick(mouse),
        Msg::MouseRelease(mouse),
        Msg::MouseWheel(mouse),
        Msg::MouseMotion(mouse),
        Msg::Paste("hi".into()),
        Msg::PasteStart,
        Msg::PasteEnd,
        Msg::Focus,
        Msg::Blur,
        Msg::CursorPosition { x: 3, y: 4 },
        Msg::KeyboardEnhancements(KeyboardEnhancementsReport { flags: 1 }),
        Msg::ModeReport {
            mode: ReportedMode::Dec(2027),
            value: ModeSetting::Set,
        },
        Msg::TerminalVersion("kitty".into()),
        Msg::ForegroundColor(Rgb::new(1, 2, 3)),
        Msg::BackgroundColor(Rgb::new(4, 5, 6)),
        Msg::CursorColor(Rgb::new(7, 8, 9)),
        Msg::WindowSize(Size::new(80, 24)),
        Msg::PixelSize(Size::new(800, 240)),
        Msg::CellSize(Size::new(10, 20)),
        Msg::ColorProfile(ColorProfile::TrueColor),
        Msg::Resume,
        Msg::Clipboard {
            content: "c".into(),
            selection: ClipboardSelection::System,
        },
        Msg::Capability("RGB".into()),
        Msg::PrimaryDeviceAttributes(vec![62, 22]),
        Msg::SecondaryDeviceAttributes(vec![1, 2]),
        Msg::TertiaryDeviceAttributes("id".into()),
        Msg::ColorScheme(ColorScheme::Dark),
    ]
}

/// Whether `msg` comes from the runtime. Matching every variant by name,
/// with no catch-all, makes a new message fail to compile here until it
/// is added to [`every_runtime_message`] and so checked to route.
fn is_runtime(msg: &Msg<u8>) -> bool {
    match msg {
        Msg::App(_) => false,
        Msg::KeyPress(_)
        | Msg::KeyRelease(_)
        | Msg::MouseClick(_)
        | Msg::MouseRelease(_)
        | Msg::MouseWheel(_)
        | Msg::MouseMotion(_)
        | Msg::Paste(_)
        | Msg::PasteStart
        | Msg::PasteEnd
        | Msg::Focus
        | Msg::Blur
        | Msg::CursorPosition { .. }
        | Msg::KeyboardEnhancements(_)
        | Msg::ModeReport { .. }
        | Msg::TerminalVersion(_)
        | Msg::ForegroundColor(_)
        | Msg::BackgroundColor(_)
        | Msg::CursorColor(_)
        | Msg::WindowSize(_)
        | Msg::PixelSize(_)
        | Msg::CellSize(_)
        | Msg::ColorProfile(_)
        | Msg::Resume
        | Msg::Clipboard { .. }
        | Msg::Capability(_)
        | Msg::PrimaryDeviceAttributes(_)
        | Msg::SecondaryDeviceAttributes(_)
        | Msg::TertiaryDeviceAttributes(_)
        | Msg::ColorScheme(_) => true,
    }
}

#[test]
fn every_runtime_message_is_checked_once() {
    let all = every_runtime_message();
    assert!(all.iter().all(is_runtime));
    let kinds: std::collections::HashSet<_> = all.iter().map(std::mem::discriminant).collect();
    assert_eq!(kinds.len(), all.len());
    assert_eq!(all.len(), 29);
}

#[test]
fn runtime_messages_pass_down_unchanged() {
    for msg in every_runtime_message() {
        let routed: Option<Msg<u16>> = msg.route(|_| Some(9));
        let text = format!("{msg:?}");
        assert_eq!(format!("{:?}", routed.expect("routed")), text);
        assert_eq!(format!("{:?}", msg.map(u16::from)), text);
    }
}

#[test]
fn the_models_own_messages_go_through_pick() {
    let mine = Msg::App(4_u8);
    assert_eq!(mine.route(|m| Some(u16::from(*m) * 2)), Some(Msg::App(8)));
    assert_eq!(mine.route(|_| None::<u16>), None);
    assert_eq!(mine.map(|m| m + 1), Msg::App(5));
}
