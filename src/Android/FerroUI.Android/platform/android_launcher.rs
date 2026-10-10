//! The launcher: opens a URI or a file with the application of the system
//! that handles it.

use super::storage::android_storage_item;
use crate::interop::java::{
    call_object, call_static_boolean, call_static_object, call_void, new_object, JavaClass, JavaLocal, JavaObject,
    JavaRef, JavaValue,
};
use crate::interop::natives::PLATFORM_HELPER;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{ILauncher, IStorageItem};
use ferroui_base::utilities::Uri;
use std::rc::Rc;

/// `Intent.ACTION_VIEW`.
const ACTION_VIEW: &str = "android.intent.action.VIEW";
/// `Intent.FLAG_ACTIVITY_CLEAR_TOP`, `FLAG_ACTIVITY_NEW_TASK` and
/// `FLAG_GRANT_READ_URI_PERMISSION`.
const FLAG_ACTIVITY_CLEAR_TOP: i32 = 0x0400_0000;
const FLAG_ACTIVITY_NEW_TASK: i32 = 0x1000_0000;
const FLAG_GRANT_READ_URI_PERMISSION: i32 = 0x0000_0001;

pub(crate) struct AndroidLauncher {
    context: JavaObject,
}

fn parse_uri(text: &str) -> Option<JavaLocal> {
    call_static_object(
        &JavaClass::find("android/net/Uri"),
        "parse",
        "(Ljava/lang/String;)Landroid/net/Uri;",
        &[JavaValue::String(text)],
    )
}

impl AndroidLauncher {
    pub fn new(context: JavaObject) -> Self {
        Self { context }
    }

    fn package_manager(&self) -> Option<JavaLocal> {
        call_object(&self.context, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[])
    }

    /// `new Intent(Intent.ActionView, uri)`.
    fn view_intent(uri: &dyn JavaRef) -> JavaLocal {
        new_object(
            &JavaClass::find("android/content/Intent"),
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JavaValue::String(ACTION_VIEW), JavaValue::Object(Some(uri))],
        )
    }

    fn set_flags(intent: &dyn JavaRef, flags: i32) {
        call_object(intent, "setFlags", "(I)Landroid/content/Intent;", &[JavaValue::Int(flags)]);
    }

    fn launch_uri(&self, uri: &Uri) -> bool {
        if uri.is_absolute_uri() && self.package_manager().is_some() {
            let Some(android_uri) = parse_uri(uri.original_string()) else {
                return false;
            };
            let intent = Self::view_intent(&android_uri);

            let flags = FLAG_ACTIVITY_CLEAR_TOP | FLAG_ACTIVITY_NEW_TASK;
            Self::set_flags(&intent, flags);
            // `StartActivity` inside the two catch clauses of the reference (no activity
            // found, a file URI exposed): the Java layer catches them and answers false.
            return call_static_boolean(
                &JavaClass::find(PLATFORM_HELPER),
                "tryStartActivity",
                "(Landroid/content/Context;Landroid/content/Intent;)Z",
                &[JavaValue::Object(Some(&self.context)), JavaValue::Object(Some(&intent))],
            );
        }
        false
    }

    fn launch_file(&self, storage_item: &Rc<dyn IStorageItem>) -> bool {
        let android_uri: Option<JavaObject> = android_storage_item::uri_of(storage_item.as_any()).or_else(|| {
            storage_item
                .try_get_local_path()
                .and_then(|local_path| parse_uri(&local_path))
                .map(|uri| uri.to_global())
        });

        if let (Some(android_uri), Some(package_manager)) = (android_uri, self.package_manager()) {
            let intent = Self::view_intent(&android_uri);
            // intent.SetDataAndType(contentUri, request.File.ContentType);
            Self::set_flags(&intent, FLAG_GRANT_READ_URI_PERMISSION);
            let resolved = call_object(
                &intent,
                "resolveActivity",
                "(Landroid/content/pm/PackageManager;)Landroid/content/ComponentName;",
                &[JavaValue::Object(Some(&package_manager))],
            );
            if resolved.is_some() {
                let empty = crate::interop::java::new_string("");
                let chooser_intent = call_static_object(
                    &JavaClass::find("android/content/Intent"),
                    "createChooser",
                    "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
                    &[JavaValue::Object(Some(&intent)), JavaValue::Object(Some(&empty))],
                );
                if let Some(chooser_intent) = chooser_intent {
                    let flags = FLAG_ACTIVITY_CLEAR_TOP | FLAG_ACTIVITY_NEW_TASK;
                    Self::set_flags(&chooser_intent, flags);
                    call_void(
                        &self.context,
                        "startActivity",
                        "(Landroid/content/Intent;)V",
                        &[JavaValue::Object(Some(&chooser_intent))],
                    );
                    return true;
                }
            }
        }
        false
    }
}

impl ILauncher for AndroidLauncher {
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(self.launch_uri(uri)))
    }

    fn launch_file_async(&self, storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(self.launch_file(&storage_item)))
    }
}
