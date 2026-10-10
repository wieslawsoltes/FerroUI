//! One face for the input context of Fcitx 4 and of Fcitx 5 (the port of
//! `FcitxICWrapper.cs`).

use super::dbus::{InputContext1Proxy, InputContextProxy};
use crate::signal_watch::watch_stream;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// The members of a formatted pre-edit: the text of a part and its format.
pub type FormattedPreedit = Vec<(String, i32)>;

#[derive(Clone)]
pub enum FcitxICWrapper {
    /// `org.fcitx.Fcitx.InputContext` (Fcitx 4).
    Old(InputContextProxy<'static>),
    /// `org.fcitx.Fcitx.InputContext1` (Fcitx 5).
    Modern(InputContext1Proxy<'static>),
}

impl FcitxICWrapper {
    pub fn from_old(old: InputContextProxy<'static>) -> Self {
        FcitxICWrapper::Old(old)
    }

    pub fn from_modern(modern: InputContext1Proxy<'static>) -> Self {
        FcitxICWrapper::Modern(modern)
    }

    pub async fn focus_in_async(&self) -> zbus::Result<()> {
        match self {
            FcitxICWrapper::Old(old) => old.focus_in().await,
            FcitxICWrapper::Modern(modern) => modern.focus_in().await,
        }
    }

    pub async fn focus_out_async(&self) -> zbus::Result<()> {
        match self {
            FcitxICWrapper::Old(old) => old.focus_out().await,
            FcitxICWrapper::Modern(modern) => modern.focus_out().await,
        }
    }

    pub async fn reset_async(&self) -> zbus::Result<()> {
        match self {
            FcitxICWrapper::Old(old) => old.reset().await,
            FcitxICWrapper::Modern(modern) => modern.reset().await,
        }
    }

    pub async fn set_cursor_rect_async(&self, x: i32, y: i32, w: i32, h: i32) -> zbus::Result<()> {
        match self {
            FcitxICWrapper::Old(old) => old.set_cursor_rect(x, y, w, h).await,
            FcitxICWrapper::Modern(modern) => modern.set_cursor_rect(x, y, w, h).await,
        }
    }

    pub async fn destroy_ic_async(&self) -> zbus::Result<()> {
        match self {
            FcitxICWrapper::Old(old) => old.destroy_ic().await,
            FcitxICWrapper::Modern(modern) => modern.destroy_ic().await,
        }
    }

    pub async fn process_key_event_async(
        &self,
        key_val: u32,
        key_code: u32,
        state: u32,
        type_: i32,
        time: u32,
    ) -> zbus::Result<bool> {
        match self {
            FcitxICWrapper::Old(old) => Ok(old.process_key_event(key_val, key_code, state, type_, time).await? != 0),
            FcitxICWrapper::Modern(modern) => modern.process_key_event(key_val, key_code, state, type_ > 0, time).await,
        }
    }

    pub async fn watch_commit_string_async(
        &self,
        handler: impl Fn(String) + 'static,
    ) -> zbus::Result<Rc<dyn IDisposable>> {
        Ok(match self {
            FcitxICWrapper::Old(old) => watch_stream(old.receive_commit_string().await?, move |signal| {
                if let Ok(args) = signal.args() {
                    handler(args.str().to_string());
                }
            }),
            FcitxICWrapper::Modern(modern) => watch_stream(modern.receive_commit_string().await?, move |signal| {
                if let Ok(args) = signal.args() {
                    handler(args.str().to_string());
                }
            }),
        })
    }

    /// The handler gets the key symbol, the state and the type (one for a
    /// release, as Fcitx 4 sends it).
    pub async fn watch_forward_key_async(
        &self,
        handler: impl Fn((u32, u32, i32)) + 'static,
    ) -> zbus::Result<Rc<dyn IDisposable>> {
        Ok(match self {
            FcitxICWrapper::Old(old) => watch_stream(old.receive_forward_key().await?, move |signal| {
                if let Ok(args) = signal.args() {
                    handler((*args.keyval(), *args.state(), *args.type_()));
                }
            }),
            FcitxICWrapper::Modern(modern) => watch_stream(modern.receive_forward_key().await?, move |signal| {
                if let Ok(args) = signal.args() {
                    handler((*args.keyval(), *args.state(), if *args.type_() { 1 } else { 0 }));
                }
            }),
        })
    }

    pub async fn watch_update_formatted_preedit_async(
        &self,
        handler: impl Fn((FormattedPreedit, i32)) + 'static,
    ) -> zbus::Result<Rc<dyn IDisposable>> {
        Ok(match self {
            FcitxICWrapper::Old(old) => watch_stream(old.receive_update_formatted_preedit().await?, move |signal| {
                if let Ok(args) = signal.args() {
                    handler((args.str().clone(), *args.cursorpos()));
                }
            }),
            FcitxICWrapper::Modern(modern) => {
                watch_stream(modern.receive_update_formatted_preedit().await?, move |signal| {
                    if let Ok(args) = signal.args() {
                        handler((args.str().clone(), *args.cursorpos()));
                    }
                })
            }
        })
    }

    pub async fn set_capacity_async(&self, flags: u32) -> zbus::Result<()> {
        match self {
            FcitxICWrapper::Old(old) => old.set_capacity(flags).await,
            FcitxICWrapper::Modern(modern) => modern.set_capability(u64::from(flags)).await,
        }
    }
}
