use super::*;

fn tick_of(t: &Model) -> TeaMsg<Msg> {
    TeaMsg::App(Msg::Tick(TickMsg {
        id: t.id(),
        timeout: false,
        tag: t.tag,
    }))
}

#[test]
fn durations_read_as_go_writes_them() {
    let cases = [
        (Duration::ZERO, "0s"),
        (Duration::from_nanos(15), "15ns"),
        (Duration::from_micros(1500), "1.5ms"),
        (Duration::from_nanos(2_500), "2.5µs"),
        (Duration::from_millis(4_999), "4.999s"),
        (Duration::from_secs(60), "1m0s"),
        (Duration::from_millis(123_250), "2m3.25s"),
        (Duration::from_secs(3600), "1h0m0s"),
        (Duration::from_secs(3725), "1h2m5s"),
    ];
    for (d, text) in cases {
        assert_eq!(go_duration(d), text);
    }
}

#[test]
fn a_timer_counts_down_its_own_ticks_and_runs_out() {
    let mut t = Model::new(Duration::from_secs(2)).with_interval(Duration::from_secs(1));
    let other = Model::new(Duration::from_secs(9));
    assert!(!t.init().is_none());
    let theirs = tick_of(&other);
    assert!(t.update(&theirs).is_none());
    assert!(!t.update(&tick_of(&t)).is_none());
    assert_eq!(t.view(), "1s");
    let stale = TeaMsg::App(Msg::Tick(TickMsg {
        id: t.id(),
        timeout: false,
        tag: 99,
    }));
    assert!(t.update(&stale).is_none());
    let _ = t.update(&tick_of(&t));
    assert!(t.timedout() && !t.running());
    assert_eq!(t.view(), "0s");
    assert!(t.update(&tick_of(&t)).is_none(), "done");
    assert!(t.start().is_none(), "a timer that ran out does not start");
}

#[test]
fn a_timer_starts_and_stops() {
    let mut t = Model::new(Duration::from_secs(5));
    assert!(!t.stop().is_none() && !t.toggle().is_none());
    let stop = TeaMsg::App(Msg::StartStop(StartStopMsg {
        id: t.id(),
        running: false,
    }));
    assert!(t.update(&stop).is_none());
    assert!(!t.running());
    assert!(t.update(&stop).is_none(), "already stopped");
    assert!(
        t.update(&tick_of(&t)).is_none(),
        "stopped timers do not tick"
    );
    let start = TeaMsg::App(Msg::StartStop(StartStopMsg {
        id: 0,
        running: true,
    }));
    assert!(!t.update(&start).is_none());
    assert!(t.running());
    let _ = TimeoutMsg { id: t.id() };
}
