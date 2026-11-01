//! Default output of the user's audio session. PipeWire and PulseAudio resolve
//! their own default device at invocation time, including after hot-plugging.
use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mixer {
    WirePlumber,
    PulseAudio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audio {
    pub mixer: Mixer,
    pub percent: u16,
    pub muted: bool,
}

fn command(program: &str, args: &[&str]) -> io::Result<String> {
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // These commands query one device and return only a few lines. A missing
    // or hung audio session must never leave an operation pending indefinitely.
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait()?.is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{program} timed out"),
            ));
        }
        thread::sleep(Duration::from_millis(8));
    }
    let result = child.wait_with_output()?;
    if !result.status.success() {
        return Err(io::Error::other(format!(
            "{program}: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&result.stdout).into_owned())
}

impl Mixer {
    fn read(self) -> Option<Audio> {
        let (percent, muted) = match self {
            Self::WirePlumber => {
                let output = command("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]).ok()?;
                let volume = output
                    .trim()
                    .strip_prefix("Volume:")?
                    .split_whitespace()
                    .next()?
                    .parse::<f64>()
                    .ok()?;
                if !volume.is_finite() || volume < 0.0 {
                    return None;
                }
                ((volume * 100.0).round() as u16, output.contains("[MUTED]"))
            }
            Self::PulseAudio => {
                let volume = command("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]).ok()?;
                let mute = command("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]).ok()?;
                let loudest = volume
                    .split_whitespace()
                    .filter_map(|s| s.strip_suffix('%')?.parse::<u16>().ok())
                    .max()?;
                (
                    loudest,
                    match mute.trim() {
                        "Mute: yes" => true,
                        "Mute: no" => false,
                        _ => return None,
                    },
                )
            }
        };
        Some(Audio {
            mixer: self,
            percent,
            muted,
        })
    }

    pub fn set_mute(self, muted: bool) -> io::Result<()> {
        let value = if muted { "1" } else { "0" };
        match self {
            Self::WirePlumber => command("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", value])?,
            Self::PulseAudio => command("pactl", &["set-sink-mute", "@DEFAULT_SINK@", value])?,
        };
        Ok(())
    }

    pub fn set_level(self, percent: u16) -> io::Result<()> {
        let percent = percent.min(100);
        match self {
            Self::WirePlumber => command(
                "wpctl",
                &[
                    "set-volume",
                    "-l",
                    "1.0",
                    "@DEFAULT_AUDIO_SINK@",
                    &format!("{percent}%"),
                ],
            )?,
            Self::PulseAudio => {
                // A relative change preserves the channel balance of stereo sinks.
                let current = self
                    .read()
                    .ok_or_else(|| io::Error::other("Default audio output disappeared"))?
                    .percent;
                command(
                    "pactl",
                    &[
                        "set-sink-volume",
                        "@DEFAULT_SINK@",
                        &format!("{:+}%", i32::from(percent) - i32::from(current)),
                    ],
                )?
            }
        };
        Ok(())
    }

    pub fn step(self, increase: bool) -> io::Result<()> {
        match self {
            Self::WirePlumber => command(
                "wpctl",
                &[
                    "set-volume",
                    "-l",
                    "1.0",
                    "@DEFAULT_AUDIO_SINK@",
                    if increase { "5%+" } else { "5%-" },
                ],
            )?,
            Self::PulseAudio => {
                let audio = self
                    .read()
                    .ok_or_else(|| io::Error::other("Default audio output disappeared"))?;
                let delta = if increase {
                    100u16.saturating_sub(audio.percent).min(5)
                } else {
                    audio.percent.min(5)
                };
                let offset = format!("{}{delta}%", if increase { '+' } else { '-' });
                command("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &offset])?
            }
        };
        Ok(())
    }
}

pub fn default_output() -> Option<Audio> {
    Mixer::WirePlumber
        .read()
        .or_else(|| Mixer::PulseAudio.read())
}
