use std::io::{self, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) struct Process {
    child: OwnedChild,
    pub(super) output: Receiver<io::Result<Vec<u8>>>,
}

impl Process {
    pub(super) fn spawn(mut command: Command, input: bool) -> Result<Self, String> {
        let mut child = OwnedChild(
            command
                .stdin(if input { Stdio::piped() } else { Stdio::null() })
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| error.to_string())?,
        );
        let mut stdout = child.0.stdout.take().ok_or("Missing voice output pipe")?;
        let (sender, output) = mpsc::sync_channel(4);
        std::thread::Builder::new()
            .name("voice-output".into())
            .spawn(move || {
                let mut bytes = [0; 4096];
                loop {
                    match stdout.read(&mut bytes) {
                        Ok(0) => break,
                        Ok(count) => {
                            if sender.send(Ok(bytes[..count].to_vec())).is_err() {
                                break;
                            }
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Err(error) => {
                            let _ = sender.send(Err(error));
                            break;
                        }
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self { child, output })
    }

    pub(super) fn write(&mut self, audio: Vec<u8>) -> Result<Receiver<io::Result<()>>, String> {
        let mut stdin = self
            .child
            .0
            .stdin
            .take()
            .ok_or("Missing voice input pipe")?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let length = u32::try_from(audio.len()).map_err(|error| error.to_string())?;
        std::thread::Builder::new()
            .name("voice-input".into())
            .spawn(move || {
                let result = stdin
                    .write_all(&wav_header(length))
                    .and_then(|()| stdin.write_all(&audio));
                drop(stdin);
                let _ = sender.send(result);
            })
            .map_err(|error| error.to_string())?;
        Ok(receiver)
    }

    pub(super) fn status(&mut self) -> Result<Option<ExitStatus>, String> {
        self.child.0.try_wait().map_err(|error| error.to_string())
    }
}

fn wav_header(length: u32) -> Vec<u8> {
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(length + 36).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16_u32.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&super::SAMPLE_RATE.to_le_bytes());
    header.extend_from_slice(&super::BYTES_PER_SECOND.to_le_bytes());
    header.extend_from_slice(&2_u16.to_le_bytes());
    header.extend_from_slice(&16_u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&length.to_le_bytes());
    header
}
