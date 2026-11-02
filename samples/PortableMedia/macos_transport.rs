//! Native NSAccessibility and AppKit keyboard transport acceptance.
use super::{
    macos_diagnostics::{transport_action, transport_key},
    native_pixels::{settle, wait},
    *,
};
async fn exercise(window: Ref<Window>, view: Ref<MediaPlayerElement>) -> Result<(), String> {
    let player = view.media_player().ok_or("No player")?;
    let observed = player.clone();
    wait(move || {
        observed.poll().map_err(|e| e.to_string())?;
        Ok((observed.playback_session().state == MediaPlaybackState::Playing).then_some(()))
    })
    .await?;
    settle().await?;
    for (label, role) in [
        ("Pause", "AXButton"),
        ("Stop", "AXButton"),
        ("Mute", "AXButton"),
        ("Playback position", "AXSlider"),
        ("Volume", "AXSlider"),
    ] {
        transport_action(&window, label, role, 0)?;
    }
    println!("MACOS_TRANSPORT_AX_PASS buttons=3 sliders=2");
    let muted = player.options().muted;
    transport_action(&window, "Mute", "AXButton", 1)?;
    if player.options().muted == muted {
        return Err("AX mute action did not reach the player".into());
    }
    transport_action(&window, "Unmute", "AXButton", 1)?;
    transport_action(&window, "Pause", "AXButton", 2)?;
    transport_key(&window, 49, " ")?;
    let observed = player.clone();
    wait(move || {
        observed.poll().map_err(|e| e.to_string())?;
        Ok((observed.playback_session().state == MediaPlaybackState::Paused).then_some(()))
    })
    .await?;
    let before_seek = player.playback_session().position;
    transport_action(&window, "Playback position", "AXSlider", 2)?;
    transport_key(&window, 124, "\u{f703}")?;
    let observed = player.clone();
    wait(move || {
        observed.poll().map_err(|e| e.to_string())?;
        Ok(
            (observed.playback_session().position > before_seek + Duration::from_millis(500))
                .then_some(()),
        )
    })
    .await?;
    transport_action(&window, "Play", "AXButton", 2)?;
    transport_key(&window, 49, " ")?;
    let observed = player.clone();
    wait(move || {
        observed.poll().map_err(|e| e.to_string())?;
        Ok((observed.playback_session().state == MediaPlaybackState::Playing).then_some(()))
    })
    .await?;
    let mut options = player.options();
    options.volume = 0.4;
    player.set_options(options).map_err(|e| e.to_string())?;
    transport_action(&window, "Volume", "AXSlider", 2)?;
    transport_key(&window, 124, "\u{f703}")?;
    if player.options().volume <= 0.4 {
        return Err("Native slider Right key did not change volume".into());
    }
    transport_action(&window, "Stop", "AXButton", 2)?;
    transport_key(&window, 49, " ")?;
    let observed = player.clone();
    wait(move || {
        observed.poll().map_err(|e| e.to_string())?;
        Ok(
            (observed.requested_state() == ferroui_media_video::PlaybackIntent::Stop
                && observed.playback_session().position < Duration::from_millis(100))
            .then_some(()),
        )
    })
    .await?;
    view.set_media_player(None, None);
    drop(player);
    println!("MACOS_TRANSPORT_KEYBOARD_PASS space_play_pause=true seek_right=true volume_right=true stop_frame_zero=true");
    Ok(())
}
pub async fn run(window: Ref<Window>, view: Ref<MediaPlayerElement>) {
    if let Err(e) = exercise(window.clone(), view).await {
        eprintln!("MACOS_TRANSPORT_FAILED {e}");
        FAILED.store(true, Ordering::Relaxed);
    }
    window.close();
}
