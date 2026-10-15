//! ArtCraft Android builder patch: a compile-only backend for `aarch64-linux-android`.
//!
//! Upstream rfd 0.17 has no Android backend, and its `FileDialog`/`MessageDialog` methods are typed
//! against these traits unconditionally, so rfd does not compile for Android at all. That breaks
//! desktop apps which call `rfd` directly (PDFCraft does, in several files) from being packaged for
//! Android.
//!
//! This module satisfies those trait bounds so the crate builds. Every dialog resolves to
//! "cancelled" (no files, no message result), so an app packaged this way launches and runs but its
//! native file pickers are inert. Apps that need working open/save on Android use a platform
//! adapter (Android's Storage Access Framework) rather than `rfd`.

use super::{
    AsyncFilePickerDialogImpl, AsyncFileSaveDialogImpl, AsyncFolderPickerDialogImpl,
    AsyncMessageDialogImpl, DialogFutureType, FilePickerDialogImpl, FileSaveDialogImpl,
    FolderPickerDialogImpl, MessageDialogImpl,
};
use crate::file_dialog::FileDialog;
use crate::message_dialog::{MessageDialog, MessageDialogResult};
use crate::FileHandle;
use std::path::PathBuf;

impl FilePickerDialogImpl for FileDialog {
    fn pick_file(self) -> Option<PathBuf> {
        None
    }

    fn pick_files(self) -> Option<Vec<PathBuf>> {
        None
    }
}

impl FileSaveDialogImpl for FileDialog {
    fn save_file(self) -> Option<PathBuf> {
        None
    }
}

impl FolderPickerDialogImpl for FileDialog {
    fn pick_folder(self) -> Option<PathBuf> {
        None
    }

    fn pick_folders(self) -> Option<Vec<PathBuf>> {
        None
    }
}

impl AsyncFilePickerDialogImpl for FileDialog {
    fn pick_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        Box::pin(async move {
            crate::artcraft_saf::pick(crate::artcraft_saf::Kind::Open).await
                .and_then(|paths| paths.into_iter().next())
                .map(FileHandle::wrap)
        })
    }

    fn pick_files_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        Box::pin(async move {
            crate::artcraft_saf::pick(crate::artcraft_saf::Kind::Open).await
                .map(|paths| paths.into_iter().map(FileHandle::wrap).collect())
        })
    }
}

impl AsyncFolderPickerDialogImpl for FileDialog {
    fn pick_folder_async(self) -> DialogFutureType<Option<FileHandle>> {
        Box::pin(async move {
            crate::artcraft_saf::pick(crate::artcraft_saf::Kind::Folder).await
                .and_then(|paths| paths.into_iter().next())
                .map(FileHandle::wrap)
        })
    }

    fn pick_folders_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        Box::pin(async move {
            crate::artcraft_saf::pick(crate::artcraft_saf::Kind::Folder).await
                .map(|paths| paths.into_iter().map(FileHandle::wrap).collect())
        })
    }
}

impl AsyncFileSaveDialogImpl for FileDialog {
    fn save_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        Box::pin(async move {
            let name = self.file_name().to_owned();
            crate::artcraft_saf::pick(crate::artcraft_saf::Kind::Save(name)).await
                .and_then(|paths| paths.into_iter().next())
                .map(FileHandle::wrap)
        })
    }
}

impl MessageDialogImpl for MessageDialog {
    fn show(self) -> MessageDialogResult {
        MessageDialogResult::Cancel
    }
}

impl AsyncMessageDialogImpl for MessageDialog {
    fn show_async(self) -> DialogFutureType<MessageDialogResult> {
        Box::pin(async { MessageDialogResult::Cancel })
    }
}
