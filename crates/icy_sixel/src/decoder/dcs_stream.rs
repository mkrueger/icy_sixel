use super::{DcsHeader, SixelDecoder, SixelFeedStatus, SixelStreamDecoder};
use crate::{Result, SixelError, SixelImage};

/// State of one complete DCS sequence after feeding a chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SixelDcsFeedStatus {
    /// All input was consumed; more bytes are required (also after a trailing ESC).
    NeedMoreData,
    /// A complete ST (`ESC \\` or 0x9c) was consumed.
    Complete,
    /// CAN (0x18) or SUB (0x1a) was consumed. Finish for the partial image, or abort.
    Cancelled(u8),
    /// Another escape or C1 control interrupted the DCS.
    ///
    /// For ESC (0x1b), the ESC **has already been consumed**, possibly in the previous
    /// chunk, but its following non-`\\` byte has not. Resume the outer ANSI parser in
    /// its escape state, or prepend ESC to the remainder. Other C1 controls are entirely
    /// unconsumed: return the remainder directly to the outer parser.
    Interrupted(u8),
}

/// Progress within a slice passed to [`SixelDcsStreamDecoder::feed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct SixelDcsFeedResult {
    /// Bytes consumed from this call, including DCS framing and ST/CAN/SUB when present.
    /// See [`SixelDcsFeedStatus::Interrupted`] for interruption byte ownership.
    pub consumed: usize,
    /// Whether this DCS needs more input or reached a terminal state.
    pub status: SixelDcsFeedStatus,
}

#[derive(Clone, Copy)]
enum Phase {
    Start,
    IntroducerEscape,
    Header,
    Payload,
    TerminatorEscape,
    End(SixelDcsFeedStatus),
    Failed,
}

/// Incremental adapter for one complete SIXEL DCS, borrowing a persistent palette owner.
///
/// Feed starting at `ESC P` or 0x90. Headers, numbers and ST can span arbitrary chunks;
/// neither the header nor the payload is buffered. This is not a general ANSI parser:
/// prefix text and non-SIXEL DCS headers are errors, not skipped. After completion or
/// interruption, subsequent feeds consume zero bytes and repeat the status.
///
/// [`Self::finish`] requires a terminal status, unlike the tolerant payload API. Errors,
/// abort and drop discard all palette changes. A successful finish commits them, including
/// changes made before cancellation. To discard cancelled graphics, call [`Self::abort`].
///
/// ```rust
/// use icy_sixel::{SixelDecoder, SixelDcsFeedStatus};
/// let mut decoder = SixelDecoder::new();
/// let mut dcs = decoder.begin_dcs();
/// assert_eq!(dcs.feed(b"\x1b")?.status, SixelDcsFeedStatus::NeedMoreData);
/// assert_eq!(dcs.feed(b"P9;1q#1;2;100;0;0!12~\x1b")?.status, SixelDcsFeedStatus::NeedMoreData);
/// let tail = b"\\terminal text";
/// let progress = dcs.feed(tail)?;
/// assert_eq!(progress.status, SixelDcsFeedStatus::Complete);
/// assert_eq!(&tail[progress.consumed..], b"terminal text");
/// assert_eq!(dcs.finish()?.dimensions(), (12, 6));
/// # Ok::<(), icy_sixel::SixelError>(())
/// ```
#[must_use = "finish the session to obtain an image and commit its palette"]
pub struct SixelDcsStreamDecoder<'a> {
    owner: Option<&'a mut SixelDecoder>,
    payload: Option<SixelStreamDecoder<'a>>,
    header: DcsHeader,
    phase: Phase,
}

impl<'a> SixelDcsStreamDecoder<'a> {
    pub(super) fn new(decoder: &'a mut SixelDecoder) -> Self {
        Self {
            owner: Some(decoder),
            payload: None,
            header: DcsHeader::new(),
            phase: Phase::Start,
        }
    }

    /// Processes a chunk, leaving any bytes after this DCS to the caller.
    ///
    /// Empty chunks do not signal EOF. An ESC at the end of a chunk is consumed and
    /// waits for lookahead to distinguish ST from another escape sequence. On error
    /// the session is poisoned and no consumed count is available; the caller must
    /// resynchronize its outer ANSI parser. No palette is committed by this method.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<SixelDcsFeedResult> {
        match self.process(bytes) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.payload = None;
                self.owner = None;
                self.phase = Phase::Failed;
                Err(error)
            }
        }
    }

    fn process(&mut self, bytes: &[u8]) -> Result<SixelDcsFeedResult> {
        let mut consumed = 0;
        loop {
            match self.phase {
                Phase::Failed => return Err(SixelError::InvalidData("DCS stream decoder has failed".into())),
                Phase::End(status) => return Ok(SixelDcsFeedResult { consumed, status }),
                _ if consumed == bytes.len() => {
                    return Ok(SixelDcsFeedResult {
                        consumed,
                        status: SixelDcsFeedStatus::NeedMoreData,
                    });
                }
                Phase::Start => {
                    self.phase = match bytes[consumed] {
                        0x1b => Phase::IntroducerEscape,
                        0x90 => Phase::Header,
                        _ => return Err(SixelError::NoSixelData),
                    };
                    consumed += 1;
                }
                Phase::IntroducerEscape => {
                    if bytes[consumed] != b'P' {
                        return Err(SixelError::NoSixelData);
                    }
                    consumed += 1;
                    self.phase = Phase::Header;
                }
                Phase::Header => {
                    let settings = self.header.push(bytes[consumed])?;
                    consumed += 1;
                    if let Some(settings) = settings {
                        self.payload = Some(self.owner.take().expect("header owns decoder").begin_frame(settings)?);
                        self.phase = Phase::Payload;
                    }
                }
                Phase::Payload => {
                    let progress = self.payload.as_mut().expect("payload initialized after header").feed(&bytes[consumed..])?;
                    consumed += progress.consumed;
                    if let SixelFeedStatus::Terminated(byte) = progress.status {
                        self.phase = match byte {
                            0x1b => {
                                consumed += 1;
                                Phase::TerminatorEscape
                            }
                            0x9c => {
                                consumed += 1;
                                Phase::End(SixelDcsFeedStatus::Complete)
                            }
                            0x18 | 0x1a => {
                                consumed += 1;
                                Phase::End(SixelDcsFeedStatus::Cancelled(byte))
                            }
                            _ => Phase::End(SixelDcsFeedStatus::Interrupted(byte)),
                        };
                    }
                }
                Phase::TerminatorEscape => {
                    self.phase = if bytes[consumed] == b'\\' {
                        consumed += 1;
                        Phase::End(SixelDcsFeedStatus::Complete)
                    } else {
                        Phase::End(SixelDcsFeedStatus::Interrupted(0x1b))
                    };
                }
            }
        }
    }

    /// Returns the image and commits its palette after completion, cancellation or interruption.
    ///
    /// EOF in the introducer, header, payload or after a lone ESC is an error and discards
    /// palette changes. For intentionally unterminated payloads use [`SixelDecoder::begin_frame`].
    pub fn finish(mut self) -> Result<SixelImage> {
        match self.phase {
            Phase::End(_) => self.payload.take().expect("terminal state has payload").finish(),
            Phase::Failed => Err(SixelError::InvalidData("DCS stream decoder has failed".into())),
            _ => Err(SixelError::InvalidData("incomplete SIXEL DCS sequence".into())),
        }
    }

    /// Discards the session and all its palette changes, equivalent to dropping it.
    pub fn abort(self) {}
}
