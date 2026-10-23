//! Host services only. The editor UI and artwork are rendered by FerroUI.
#[cfg(not(target_os = "emscripten"))]
static NATIVE_SAVE_SUCCEEDED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);
#[cfg(not(target_os = "emscripten"))]
static NATIVE_SERIALIZATION_PENDING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
#[cfg(not(target_os = "emscripten"))]
static NATIVE_DRAFT_DIRTY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
#[cfg(target_os = "emscripten")]
mod browser {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(inline_js = r#"
export function ferroLoad() { return globalThis.ferroDesignStorage?.load() || ''; }
export function ferroSave(text) { return globalThis.ferroDesignStorage?.save(text) === true; }
export function ferroSerializationPending(pending) { globalThis.ferroDesignStorage?.setSerializationPending(pending); }
export function ferroDraftDirty(dirty) { globalThis.ferroDesignStorage?.setDraftDirty(dirty); }
export function ferroSaveStatus() {
  switch(globalThis.ferroDesignStorage?.status().phase) {
    case 'saved': return 0;
    case 'saving': return 1;
    case 'conflict': return 3;
    case 'editing': return 4;
    default: return 2;
  }
}
export function ferroDownload(name, text, mime) {
  const url = URL.createObjectURL(new Blob([text], {type:mime}));
  const a = document.createElement('a'); a.href=url; a.download=name; a.click();
  setTimeout(()=>URL.revokeObjectURL(url),10000);
}
export function ferroCopy(text) { navigator.clipboard?.writeText(text).catch(()=>{}); }
export function ferroOpenUrl(value) {
  let url;
  try { url = new URL(value); } catch { return false; }
  if (!['http:', 'https:', 'mailto:', 'tel:'].includes(url.protocol)) return false;
  // noopener in window.open's feature string deliberately returns null even
  // when opening succeeds. Open a controlled blank page so popup blocking can
  // be detected, then sever the opener before any external content can load.
  let child;
  try { child = window.open('about:blank', '_blank'); } catch { return false; }
  if (!child) return false;
  try {
    child.opener = null;
    const policy = child.document.createElement('meta');
    policy.name = 'referrer'; policy.content = 'no-referrer';
    child.document.head.appendChild(policy);
    child.location.replace(url.href);
    return true;
  } catch {
    try { child.close(); } catch {}
    return false;
  }
}
export function ferroPick(accept) {
  const picker=document.createElement('input'); picker.type='file'; picker.accept=accept;
  picker.onchange=async()=>{const f=picker.files?.[0]; if(!f)return;
    const api=globalThis.ferroDesign;
    try {
      if(f.size>48*1024*1024) throw new Error('The selected file exceeds 48 MiB.');
      const raster=/^image\/(png|jpeg|gif|bmp)$/i.test(f.type);
      const text=raster ? await new Promise((resolve,reject)=>{
        const reader=new FileReader(); reader.onload=()=>resolve(reader.result);
        reader.onerror=()=>reject(new Error('The image could not be read.'));
        reader.readAsDataURL(f);
      }) : await f.text();
      if(api?.ferroDesignImportFile) api.ferroDesignImportFile(text,f.name);
      else if(api?.ferroDesignImportJson) api.ferroDesignImportJson(text);
    } catch(error) { api?.ferroDesignHostError?.(String(error.message||error)); }
  }; picker.click();
}
export function ferroBytes(name, bytes, mime) {
  const url=URL.createObjectURL(new Blob([bytes],{type:mime}));
  const a=document.createElement('a');a.href=url;a.download=name;a.click();
  setTimeout(()=>URL.revokeObjectURL(url),10000);
}
"#)]
    extern "C" {
        #[wasm_bindgen(js_name=ferroLoad)]
        pub fn load() -> String;
        #[wasm_bindgen(js_name=ferroSave)]
        pub fn save(text: &str) -> bool;
        #[wasm_bindgen(js_name=ferroSaveStatus)]
        pub fn save_status() -> u8;
        #[wasm_bindgen(js_name=ferroSerializationPending)]
        pub fn serialization_pending(pending: bool);
        #[wasm_bindgen(js_name=ferroDraftDirty)]
        pub fn draft_dirty(dirty: bool);
        #[wasm_bindgen(js_name=ferroDownload)]
        pub fn download(name: &str, text: &str, mime: &str);
        #[wasm_bindgen(js_name=ferroCopy)]
        pub fn copy(text: &str);
        #[wasm_bindgen(js_name=ferroOpenUrl)]
        pub fn open_url(url: &str) -> bool;
        #[wasm_bindgen(js_name=ferroPick)]
        pub fn pick(accept: &str);
        #[wasm_bindgen(js_name=ferroBytes)]
        pub fn bytes(name: &str, bytes: &[u8], mime: &str);
    }
}
pub fn load_document() -> String {
    #[cfg(target_os = "emscripten")]
    {
        browser::load()
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        std::fs::read_to_string("ferrodesign-autosave.json").unwrap_or_default()
    }
}
pub fn save_document(text: &str) -> bool {
    #[cfg(target_os = "emscripten")]
    {
        browser::save(text)
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        let saved =
            write_document_atomically(std::path::Path::new("ferrodesign-autosave.json"), text)
                .is_ok();
        NATIVE_SAVE_SUCCEEDED.store(saved, std::sync::atomic::Ordering::Relaxed);
        saved
    }
}
/// Keep lifecycle guards active before a coalesced JSON snapshot is produced.
pub fn set_document_serialization_pending(pending: bool) {
    #[cfg(target_os = "emscripten")]
    browser::serialization_pending(pending);
    #[cfg(not(target_os = "emscripten"))]
    NATIVE_SERIALIZATION_PENDING.store(pending, std::sync::atomic::Ordering::Relaxed);
}
/// The browser guards an active native text draft without serializing the whole
/// design on every keystroke. Lifecycle events commit it through the shell once.
pub fn set_document_draft_dirty(dirty: bool) {
    #[cfg(target_os = "emscripten")]
    browser::draft_dirty(dirty);
    #[cfg(not(target_os = "emscripten"))]
    NATIVE_DRAFT_DIRTY.store(dirty, std::sync::atomic::Ordering::Relaxed);
}
/// A browser save request is queued synchronously; its status changes to saved
/// only when IndexedDB completes the entire transaction.
pub fn document_save_status() -> &'static str {
    #[cfg(target_os = "emscripten")]
    {
        match browser::save_status() {
            0 => "All changes saved locally",
            1 => "Saving locally…",
            3 => "Newer save in another tab · Export before reloading",
            4 => "Editing text…",
            _ => "Save failed · Export to keep your changes",
        }
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        use std::sync::atomic::Ordering;
        if !NATIVE_SAVE_SUCCEEDED.load(Ordering::Relaxed) {
            "Save failed · Export to keep your changes"
        } else if NATIVE_DRAFT_DIRTY.load(Ordering::Relaxed) {
            "Editing text…"
        } else if NATIVE_SERIALIZATION_PENDING.load(Ordering::Relaxed) {
            "Saving locally…"
        } else {
            "All changes saved locally"
        }
    }
}
#[cfg(not(target_os = "emscripten"))]
fn write_document_atomically(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_SAVE: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "The autosave path must name a file.",
        )
    })?;
    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT_SAVE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut created = false;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        created = true;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        // The temporary file shares the destination filesystem. Atomic replace
        // keeps the last complete document intact if writing is interrupted.
        std::fs::rename(&temporary, path)?;
        #[cfg(unix)]
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if created && result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
pub fn download(name: &str, text: &str, mime: &str) {
    #[cfg(target_os = "emscripten")]
    browser::download(name, text, mime);
    #[cfg(not(target_os = "emscripten"))]
    {
        let _ = mime;
        if let Err(e) = std::fs::write(name, text) {
            eprintln!("Cannot save {name}: {e}");
        }
    }
}
pub fn copy(text: &str) {
    #[cfg(target_os = "emscripten")]
    browser::copy(text);
    #[cfg(not(target_os = "emscripten"))]
    {
        use std::io::Write;
        let commands: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
            &[("pbcopy", &[])]
        } else if cfg!(target_os = "windows") {
            &[("clip", &[])]
        } else {
            &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])]
        };
        for (cmd, args) in commands {
            if let Ok(mut child) = std::process::Command::new(cmd)
                .args(*args)
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if child.wait().is_ok_and(|s| s.success()) {
                    break;
                }
            }
        }
    }
}
/// Requests an external URL through the host. Success means the browser accepted
/// the navigation or the desktop opener exited successfully; it cannot confirm
/// that a remote page loaded or that a mail/telephone handler completed its work.
pub fn open_url(url: &str) -> bool {
    let Some(url) = ferrodesign_core::prototype_external_url(url) else {
        return false;
    };
    #[cfg(target_os = "emscripten")]
    {
        browser::open_url(&url)
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        use std::process::{Command, Stdio};
        let mut command = if cfg!(target_os = "macos") {
            Command::new("open")
        } else if cfg!(target_os = "windows") {
            let mut command = Command::new("rundll32.exe");
            command.arg("url.dll,FileProtocolHandler");
            command
        } else {
            Command::new("xdg-open")
        };
        command
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}
pub fn pick_file() {
    #[cfg(target_os = "emscripten")]
    browser::pick(".ferrodesign,.json,.svg,image/png,image/jpeg,image/gif,image/bmp");
    #[cfg(not(target_os = "emscripten"))]
    eprintln!("Use the Import JSON dialog to open a design on desktop.");
}
pub fn pick_image() {
    #[cfg(target_os = "emscripten")]
    browser::pick("image/png,image/jpeg,image/gif,image/bmp");
    #[cfg(not(target_os = "emscripten"))]
    eprintln!("Paste a raster image data URI in the Import dialog to place an image.");
}
pub fn download_bytes(name: &str, bytes: &[u8], mime: &str) -> bool {
    #[cfg(target_os = "emscripten")]
    {
        browser::bytes(name, bytes, mime);
        true
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        let _ = mime;
        std::fs::write(name, bytes).is_ok()
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod persistence_tests {
    use super::write_document_atomically;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "ferrodesign-autosave-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn atomic_autosave_replaces_the_complete_unicode_document() {
        let directory = TestDirectory::new();
        let path = directory.0.join("design.json");
        write_document_atomically(&path, "original document").unwrap();
        let replacement = "Orbit · café Ω\n".repeat(100_000);
        write_document_atomically(&path, &replacement).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), replacement);
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }
    #[test]
    fn failed_atomic_replace_preserves_existing_data_and_cleans_its_temporary_file() {
        let directory = TestDirectory::new();
        let destination = directory.0.join("existing-directory");
        std::fs::create_dir(&destination).unwrap();
        let original = destination.join("original.json");
        std::fs::write(&original, "previous saved document").unwrap();
        assert!(write_document_atomically(&destination, "replacement").is_err());
        assert_eq!(
            std::fs::read_to_string(original).unwrap(),
            "previous saved document"
        );
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }
}
