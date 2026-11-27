use super::*;
use std::time::UNIX_EPOCH;

fn at(ms: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms)
}

#[test]
fn a_stopwatch_counts_between_its_own_messages() {
    let mut s = Model::default().with_interval(Duration::from_millis(100));
    let id = s.id();
    assert!(!s.init().is_none() && !s.toggle().is_none() && !s.reset().is_none());
    let _ = s.update(&TeaMsg::App(Msg::StartStop {
        id,
        running: true,
        at: at(0),
    }));
    assert!(s.running());
    assert!(!s.toggle().is_none());
    let tick = TeaMsg::App(Msg::Tick {
        id,
        at: at(130),
        tag: 0,
    });
    assert!(!s.update(&tick).is_none());
    assert_eq!(s.elapsed(), Duration::from_millis(130));
    assert_eq!(s.view(), "100ms");
    let stale = TeaMsg::App(Msg::Tick {
        id,
        at: at(140),
        tag: 7,
    });
    assert!(s.update(&stale).is_none());
    let other = TeaMsg::App(Msg::Tick {
        id: id + 1000,
        at: at(150),
        tag: 0,
    });
    assert!(s.update(&other).is_none());
    let _ = s.update(&TeaMsg::App(Msg::StartStop {
        id,
        running: false,
        at: at(250),
    }));
    assert!(!s.running());
    assert_eq!(s.elapsed(), Duration::from_millis(250));
    assert!(s.update(&tick).is_none(), "stopped");
    let _ = s.update(&TeaMsg::App(Msg::Reset { id, at: at(300) }));
    assert_eq!(s.view(), "0s");
    s.interval = Duration::ZERO;
    assert_eq!(s.view(), "0s");
}
