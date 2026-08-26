use anyhow::{Result, anyhow};
use futures::channel::oneshot;
use gpui::PathPromptOptions;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use std::{
    ffi::CStr,
    path::{Path, PathBuf},
    sync::{
        Mutex, Once, OnceLock,
        atomic::{AtomicPtr, Ordering},
    },
};

enum PendingPicker {
    Open(oneshot::Sender<Result<Option<Vec<PathBuf>>>>),
    Save {
        sender: oneshot::Sender<Result<Option<PathBuf>>>,
        temporary_path: PathBuf,
    },
}

impl PendingPicker {
    fn send_result(self, result: Option<Vec<PathBuf>>) {
        match self {
            Self::Open(sender) => {
                let _ = sender.send(Ok(result));
            }
            Self::Save {
                sender,
                temporary_path,
            } => {
                let _ = std::fs::remove_file(temporary_path);
                let _ = sender.send(Ok(result.and_then(|paths| paths.into_iter().next())));
            }
        }
    }

    fn send_error(self, error: anyhow::Error) {
        match self {
            Self::Open(sender) => {
                let _ = sender.send(Err(error));
            }
            Self::Save {
                sender,
                temporary_path,
            } => {
                let _ = std::fs::remove_file(temporary_path);
                let _ = sender.send(Err(error));
            }
        }
    }
}

static PENDING_PICKER: Mutex<Option<PendingPicker>> = Mutex::new(None);
static PICKER_DELEGATE: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());
/// `UIDocumentPickerModeImport`: UIKit copies the selected resource into this
/// app's sandbox, so the returned path remains usable after the delegate call.
const UIDOCUMENT_PICKER_MODE_IMPORT: isize = 0;
static REGISTER_DELEGATE: Once = Once::new();
static DELEGATE_CLASS: OnceLock<&'static Class> = OnceLock::new();

fn ns_string(value: &str) -> *mut Object {
    unsafe { super::ns_string_from_str(value) }
}

fn string_from_ns(value: *mut Object) -> Option<String> {
    if value.is_null() {
        return None;
    }
    unsafe {
        let bytes: *const std::ffi::c_char = msg_send![value, UTF8String];
        (!bytes.is_null()).then(|| CStr::from_ptr(bytes).to_string_lossy().into_owned())
    }
}

fn finish(result: Option<Vec<PathBuf>>) {
    let pending = PENDING_PICKER.lock().unwrap().take();
    if let Some(pending) = pending {
        pending.send_result(result);
    }
    release_delegate();
}

fn release_delegate() {
    let delegate = PICKER_DELEGATE.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !delegate.is_null() {
        unsafe {
            let _: () = msg_send![delegate, release];
        }
    }
}

fn fail_pending(error: anyhow::Error) {
    if let Some(pending) = PENDING_PICKER.lock().unwrap().take() {
        pending.send_error(error);
    }
    release_delegate();
}

fn delegate_class() -> &'static Class {
    REGISTER_DELEGATE.call_once(|| {
        let mut declaration =
            ClassDecl::new("GPUIDocumentPickerDelegate", class!(NSObject)).unwrap();
        if let Some(protocol) = Protocol::get("UIDocumentPickerDelegate") {
            declaration.add_protocol(protocol);
        }

        extern "C" fn did_pick(_this: &Object, _sel: Sel, _picker: *mut Object, urls: *mut Object) {
            let mut paths = Vec::new();
            if !urls.is_null() {
                unsafe {
                    let count: usize = msg_send![urls, count];
                    for index in 0..count {
                        let url: *mut Object = msg_send![urls, objectAtIndex: index];
                        let path: *mut Object = msg_send![url, path];
                        if let Some(path) = string_from_ns(path) {
                            paths.push(PathBuf::from(path));
                        }
                    }
                }
            }
            finish(Some(paths));
        }

        extern "C" fn cancelled(_this: &Object, _sel: Sel, _picker: *mut Object) {
            finish(None);
        }

        unsafe {
            declaration.add_method(
                sel!(documentPicker:didPickDocumentsAtURLs:),
                did_pick as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
            );
            declaration.add_method(
                sel!(documentPickerWasCancelled:),
                cancelled as extern "C" fn(&Object, Sel, *mut Object),
            );
        }
        let _ = DELEGATE_CLASS.set(declaration.register());
    });
    DELEGATE_CLASS.get().copied().unwrap()
}

unsafe fn present(picker: *mut Object, pending: PendingPicker) {
    if picker.is_null() {
        pending.send_error(anyhow!(
            "UIKit failed to create UIDocumentPickerViewController"
        ));
        return;
    }
    let mut guard = PENDING_PICKER.lock().unwrap();
    if guard.is_some() {
        let _: () = msg_send![picker, release];
        pending.send_error(anyhow!("a document picker is already active"));
        return;
    }

    let delegate: *mut Object = msg_send![delegate_class(), new];
    let _: () = msg_send![picker, setDelegate: delegate];
    PICKER_DELEGATE.store(delegate, Ordering::Release);
    *guard = Some(pending);
    drop(guard);

    let application: *mut Object = msg_send![class!(UIApplication), sharedApplication];
    let key_window: *mut Object = msg_send![application, keyWindow];
    let mut controller: *mut Object = msg_send![key_window, rootViewController];
    while !controller.is_null() {
        let presented: *mut Object = msg_send![controller, presentedViewController];
        if presented.is_null() {
            break;
        }
        controller = presented;
    }
    if controller.is_null() {
        let _: () = msg_send![picker, release];
        fail_pending(anyhow!(
            "no active UIKit view controller for document picker"
        ));
        return;
    }
    let _: () = msg_send![
        controller,
        presentViewController: picker
        animated: true
        completion: std::ptr::null::<Object>()
    ];
    let _: () = msg_send![picker, release];
}

pub fn prompt_for_paths(
    options: PathPromptOptions,
) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
    let (sender, receiver) = oneshot::channel();
    unsafe {
        let types: *mut Object = msg_send![class!(NSMutableArray), array];
        if options.files {
            let _: () = msg_send![types, addObject: ns_string("public.data")];
        }
        if options.directories {
            let _: () = msg_send![types, addObject: ns_string("public.folder")];
        }
        if !options.files && !options.directories {
            let _ = sender.send(Err(anyhow!("path prompt must allow files or directories")));
            return receiver;
        }
        let picker: *mut Object = msg_send![class!(UIDocumentPickerViewController), alloc];
        let picker: *mut Object =
            msg_send![picker, initWithDocumentTypes: types inMode: UIDOCUMENT_PICKER_MODE_IMPORT];
        let _: () = msg_send![picker, setAllowsMultipleSelection: options.multiple];
        present(picker, PendingPicker::Open(sender));
    }
    receiver
}

pub fn prompt_for_new_path(
    directory: &Path,
    suggested_name: Option<&str>,
) -> oneshot::Receiver<Result<Option<PathBuf>>> {
    let (sender, receiver) = oneshot::channel();
    let file_name = suggested_name
        .and_then(|name| Path::new(name).file_name())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| std::ffi::OsStr::new("Untitled"));
    let temporary_path = std::env::temp_dir().join(file_name);
    if let Err(error) = std::fs::write(&temporary_path, []) {
        let _ = sender.send(Err(error.into()));
        return receiver;
    }

    unsafe {
        let url: *mut Object = msg_send![
            class!(NSURL),
            fileURLWithPath: ns_string(&temporary_path.to_string_lossy())
        ];
        let urls: *mut Object = msg_send![class!(NSArray), arrayWithObject: url];
        let picker: *mut Object = msg_send![class!(UIDocumentPickerViewController), alloc];
        let picker: *mut Object = msg_send![picker, initWithURLs: urls inMode: 3_isize];
        let directory_url: *mut Object = msg_send![
            class!(NSURL),
            fileURLWithPath: ns_string(&directory.to_string_lossy())
        ];
        let responds: bool = msg_send![picker, respondsToSelector: sel!(setDirectoryURL:)];
        if responds {
            let _: () = msg_send![picker, setDirectoryURL: directory_url];
        }
        present(
            picker,
            PendingPicker::Save {
                sender,
                temporary_path,
            },
        );
    }
    receiver
}
