//! Mouse events print as Bubble Tea's do.

use super::*;

fn mouse(button: MouseButton, mods: KeyMod) -> Mouse {
    Mouse {
        x: 3,
        y: 4,
        button,
        mods,
    }
}

#[test]
fn mouse_events_print_modifiers_then_the_button() {
    assert_eq!(mouse(MouseButton::Left, KeyMod::NONE).to_string(), "left");
    assert_eq!(
        mouse(MouseButton::Left, KeyMod::CTRL).to_string(),
        "ctrl+left"
    );
    let all = KeyMod::SHIFT | KeyMod::ALT | KeyMod::CTRL;
    assert_eq!(
        mouse(MouseButton::WheelUp, all).to_string(),
        "ctrl+alt+shift+wheelup"
    );
    assert_eq!(
        mouse(MouseButton::Button11, KeyMod::NONE).to_string(),
        "button11"
    );
    assert_eq!(mouse(MouseButton::None, KeyMod::NONE).to_string(), "");
}

#[test]
fn motion_says_motion_with_or_without_a_button() {
    assert_eq!(
        mouse(MouseButton::Left, KeyMod::NONE).motion_string(),
        "left+motion"
    );
    assert_eq!(
        mouse(MouseButton::None, KeyMod::NONE).motion_string(),
        "motion"
    );
    assert_eq!(
        mouse(MouseButton::None, KeyMod::ALT).motion_string(),
        "alt+motion"
    );
}

#[test]
fn only_the_four_wheel_directions_are_wheels() {
    let wheels = [
        MouseButton::WheelUp,
        MouseButton::WheelDown,
        MouseButton::WheelLeft,
        MouseButton::WheelRight,
    ];
    assert!(wheels.iter().all(|b| b.is_wheel()));
    assert!(!MouseButton::Middle.is_wheel() && !MouseButton::Backward.is_wheel());
    assert_eq!(MouseButton::Forward.to_string(), "forward");
}
