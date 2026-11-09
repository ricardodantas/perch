//! Image loading and caching for terminal display.
//!
//! This module handles downloading, caching, and preparing images for
//! rendering in the terminal using various graphics protocols (Sixel,
//! Kitty, iTerm2) with fallback to Unicode halfblocks.

mod cache;
mod loader;

pub use cache::ImageCache;
pub use loader::ImageLoader;

use ratatui_image::picker::Picker;
use std::sync::OnceLock;
#[cfg(unix)]
use std::time::{Duration, Instant};

/// Global picker instance (initialized once at startup)
static PICKER: OnceLock<Option<Picker>> = OnceLock::new();

/// How long to wait for the terminal to answer a status query before treating
/// it as one that doesn't answer queries (and falling back to halfblocks).
#[cfg(unix)]
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Initialize the image picker by querying terminal capabilities.
///
/// Call once at startup, after raw mode is enabled and the alternate screen is
/// entered, and before terminal events are read: the query responses arrive on
/// stdin. Returns `true` if a picker is available (halfblocks at worst).
pub fn init_picker() -> bool {
    let picker = PICKER.get_or_init(|| {
        // ratatui-image reads its query responses on a detached thread. If the
        // terminal never answers, that thread stays blocked on stdin after the
        // timeout and swallows the user's first keystroke, so only run the
        // query once the terminal has shown it answers queries.
        if !terminal_answers_queries() {
            tracing::debug!("Terminal did not answer a status query; using halfblocks");
            return Some(Picker::halfblocks());
        }

        // Try to detect terminal graphics support
        // This queries the terminal for sixel/kitty/iterm2 support
        match Picker::from_query_stdio() {
            Ok(p) => {
                tracing::info!("Image support detected: {:?}", p.protocol_type());
                Some(p)
            }
            Err(e) => {
                tracing::debug!("No image protocol support: {e}");
                // Fall back to halfblocks (Unicode-based, works everywhere)
                Some(Picker::halfblocks())
            }
        }
    });
    picker.is_some()
}

/// Send a Device Status Report query (`ESC [ 5 n`) and wait up to
/// [`PROBE_TIMEOUT`] for the reply, without leaving a reader behind.
///
/// Any terminal that answers queries replies to DSR, and ratatui-image ends its
/// own query with DSR, so a terminal that passes this probe also lets the
/// library's reader thread finish. Bytes read before the reply (typeahead) are
/// discarded, as the library's query would do.
#[cfg(unix)]
fn terminal_answers_queries() -> bool {
    use rustix::event::{PollFd, PollFlags, Timespec};
    use rustix::io::Errno;
    use std::io::{IsTerminal, Write};

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    if !stdin.is_terminal() || !stdout.is_terminal() {
        return false;
    }
    if stdout
        .write_all(b"\x1b[5n")
        .and_then(|()| stdout.flush())
        .is_err()
    {
        return false;
    }

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut received = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        let Ok(timeout) = Timespec::try_from(remaining) else {
            return false;
        };
        let mut fds = [PollFd::new(&stdin, PollFlags::IN)];
        match rustix::event::poll(&mut fds, Some(&timeout)) {
            Ok(0) => return false,
            Ok(_) => {}
            Err(Errno::INTR) => continue,
            Err(_) => return false,
        }
        // One byte at a time on the raw fd: don't consume input past the reply.
        let mut byte = [0u8; 1];
        match rustix::io::read(&stdin, &mut byte) {
            Ok(1) => {
                received.push(byte[0]);
                if ends_with_status_reply(&received) {
                    return true;
                }
            }
            Err(Errno::INTR | Errno::AGAIN) => {}
            // EOF or a read error
            _ => return false,
        }
    }
}

/// On non-Unix platforms keep querying unconditionally (previous behaviour).
#[cfg(not(unix))]
const fn terminal_answers_queries() -> bool {
    true
}

/// Get the global picker instance.
pub fn picker() -> Option<&'static Picker> {
    PICKER.get().and_then(|p| p.as_ref())
}

/// Check if terminal supports native graphics (not just halfblocks).
pub fn has_native_graphics() -> bool {
    picker().is_some_and(|p| {
        !matches!(
            p.protocol_type(),
            ratatui_image::picker::ProtocolType::Halfblocks
        )
    })
}

/// Whether `buf` ends with a Device Status Report reply (`ESC [ <digits> n`).
fn ends_with_status_reply(buf: &[u8]) -> bool {
    let Some(rest) = buf.strip_suffix(b"n") else {
        return false;
    };
    let digits = rest.iter().rev().take_while(|b| b.is_ascii_digit()).count();
    rest[..rest.len() - digits].ends_with(b"\x1b[")
}

#[cfg(test)]
mod tests {
    use super::ends_with_status_reply;

    #[test]
    fn status_reply_is_recognised() {
        assert!(ends_with_status_reply(b"\x1b[0n"));
    }

    #[test]
    fn status_reply_after_typeahead_is_recognised() {
        assert!(ends_with_status_reply(b"jk\x1b[?62;4c\x1b[0n"));
    }

    #[test]
    fn incomplete_or_unrelated_input_is_not_a_status_reply() {
        for buf in [
            &b""[..],
            b"n",
            b"\x1b[0",
            b"\x1b[0c",
            b"[0n",
            b"\x1b[0xn",
            b"q",
        ] {
            assert!(!ends_with_status_reply(buf), "{buf:?}");
        }
    }
}
