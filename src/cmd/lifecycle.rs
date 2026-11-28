//! Commands that hand the terminal to something else and take it back:
//! running another program, suspending, releasing, and repainting after
//! the screen was written over.

use super::{Cmd, Handover, Inner, Output};
use std::io;
use std::process::{Command, ExitStatus};

/// Runs `command` in the terminal, as Bubble Tea's `tea.ExecProcess`
/// does, then sends `done(result)`: for an editor, a pager or a shell.
///
/// The program hands the terminal over on its own thread, in program
/// order: it stops reading input (bytes not yet read stay for the
/// child), turns every mode it set off, leaves the alternate screen,
/// erases an inline frame and puts the terminal's own mode back. The
/// child inherits the program's standard input, output and error. When
/// it exits, the program takes the terminal back: raw mode, every mode
/// on again, the whole frame drawn again, input read again; `done`'s
/// message is the next one the model sees. Keys typed while the child
/// runs are the child's; none are lost or read twice (Bubble Tea issues
/// #616 and #1424). Ctrl-Z while the child has the terminal stops both,
/// as in a shell.
///
/// ```
/// use petitcarnets as tea;
/// use std::process::Command;
///
/// enum Msg {
///     EditorClosed(std::io::Result<std::process::ExitStatus>),
/// }
///
/// struct Editor(Option<bool>);
///
/// impl tea::Model for Editor {
///     type Msg = Msg;
///     fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
///         match msg {
///             // `vim` in a real program; `true` here, which ends at once.
///             tea::Msg::KeyPress(k) if k == "e" => tea::exec(Command::new("true"), Msg::EditorClosed),
///             tea::Msg::App(Msg::EditorClosed(status)) => {
///                 self.0 = Some(status.is_ok_and(|s| s.success()));
///                 tea::quit()
///             }
///             _ => tea::Cmd::none(),
///         }
///     }
///     fn view(&self) -> tea::View {
///         "e: edit".into()
///     }
/// }
///
/// let mut test = tea::testing::TestProgram::new(Editor(None), 20, 2);
/// test.type_text("e");
/// assert_eq!(test.final_model().0, Some(true));
/// ```
#[must_use]
pub fn exec<M>(
    command: Command,
    done: impl FnOnce(io::Result<ExitStatus>) -> M + Send + 'static,
) -> Cmd<M> {
    let mut command = command;
    Cmd(Inner::Handover(Handover::Exec(Box::new(
        move |failed: Option<io::Error>| done(failed.map_or_else(|| command.status(), Err)),
    ))))
}

/// Something to run in the terminal with [`exec_command`]: Bubble Tea's
/// `tea.ExecCommand`. It runs on the program's thread while the program
/// has handed the terminal over, and reads and writes the terminal
/// through the process's standard streams.
pub trait ExecCommand: Send {
    /// Runs to the end.
    ///
    /// # Errors
    ///
    /// Whatever went wrong; it is passed to the callback.
    fn run(&mut self) -> io::Result<()>;
}

/// A [`Command`] succeeds when it exits with status 0.
impl ExecCommand for Command {
    fn run(&mut self) -> io::Result<()> {
        let status = self.status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("{status}")))
        }
    }
}

/// Runs `command` in the terminal as [`exec`] runs a [`Command`], then
/// sends `done(result)`: Bubble Tea's `tea.Exec`, for anything that
/// needs the terminal to itself.
#[must_use]
pub fn exec_command<M>(
    command: impl ExecCommand + 'static,
    done: impl FnOnce(io::Result<()>) -> M + Send + 'static,
) -> Cmd<M> {
    let mut command = command;
    Cmd(Inner::Handover(Handover::Exec(Box::new(
        move |failed: Option<io::Error>| done(failed.map_or_else(|| command.run(), Err)),
    ))))
}

/// Suspends the program as Ctrl-Z in a shell suspends one: the terminal
/// is handed back as for [`exec`], the process (and its process group)
/// stops, and when the shell continues it (`fg`), the program takes the
/// terminal back, draws the whole frame and sends
/// [`Msg::Resume`](crate::Msg::Resume). Bubble Tea's `tea.Suspend`.
///
/// Ctrl-Z does this by default; see
/// [`Program::without_suspend`](crate::Program::without_suspend).
#[must_use]
pub const fn suspend<M>() -> Cmd<M> {
    Cmd(Inner::Handover(Handover::Suspend))
}

/// Hands the terminal over as [`exec`] does and keeps it handed over,
/// while the program goes on updating the model, until
/// [`restore_terminal`]. Nothing is drawn meanwhile, and output from
/// [`println`](crate::println), [`raw`](crate::raw) and the like waits.
/// Bubble Tea's `Program.ReleaseTerminal`.
///
/// Go's is a method callable from any goroutine; here the terminal
/// belongs to the program's thread, so this is a command, run in program
/// order: `sequence([release_terminal(), Cmd::task(work),
/// restore_terminal()])` runs `work` with the terminal to itself.
#[must_use]
pub const fn release_terminal<M>() -> Cmd<M> {
    Cmd(Inner::Handover(Handover::Release))
}

/// Takes the terminal back after [`release_terminal`]: raw mode, every
/// mode on again, the whole frame drawn again. Bubble Tea's
/// `Program.RestoreTerminal`. Does nothing if the terminal is not
/// released.
#[must_use]
pub const fn restore_terminal<M>() -> Cmd<M> {
    Cmd(Inner::Handover(Handover::Restore))
}

/// Turns every mode the program set off and on again and draws the whole
/// frame, for when something else wrote over the terminal: Bubble Tea's
/// `tea.Repaint` (its issue #1808). Unlike [`clear_screen`](crate::clear_screen)
/// it enters the alternate screen again if something left it, and
/// redraws even when the view has not changed.
#[must_use]
pub const fn repaint<M>() -> Cmd<M> {
    Cmd(Inner::Output(Output::Repaint))
}
