//! The libc calls, on bad descriptors and real pipes.

use super::*;

#[test]
fn a_bad_descriptor_is_not_a_terminal() {
    assert!(!is_tty(-1));
    assert!(get_mode(-1).is_err());
    assert!(window_size(-1).is_err());
}

#[test]
fn the_opaque_buffer_holds_any_termios() {
    assert!(size_of::<Termios>() >= 72);
    assert_eq!(align_of::<Termios>(), 8);
}

#[test]
fn a_pipe_carries_bytes_and_wakes_a_wait() {
    let pipe = Pipe::new().unwrap();
    write_all_fd(pipe.write, b"hi").unwrap();
    assert_eq!(wait_readable([pipe.read], None).unwrap(), [true]);
    let mut buf = [0u8; 8];
    assert_eq!(read_fd(pipe.read, &mut buf).unwrap(), 2);
    assert_eq!(&buf[..2], b"hi");
}

#[test]
fn a_wait_ends_at_its_timeout() {
    let pipe = Pipe::new().unwrap();
    let start = Instant::now();
    let ready = wait_readable([pipe.read], Some(Duration::from_millis(20))).unwrap();
    assert_eq!(ready, [false]);
    assert!(start.elapsed() >= Duration::from_millis(20));
}

#[test]
fn the_write_end_never_blocks() {
    let pipe = Pipe::new().unwrap();
    let chunk = [0u8; 4096];
    let full = (0..1024).find_map(|_| write_all_fd(pipe.write, &chunk).err());
    let err = full.expect("a pipe fills up long before 4 MiB");
    assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
}
