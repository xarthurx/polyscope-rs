use std::sync::Mutex;

use crate::ScreenshotOptions;

/// Global screenshot request storage.
/// This allows `screenshot()` to be called from user code while `show()` is running.
static SCREENSHOT_REQUEST: Mutex<Option<ScreenshotRequest>> = Mutex::new(None);

/// A pending screenshot request.
#[derive(Debug, Clone)]
pub struct ScreenshotRequest {
    /// Filename to save to. None means auto-generate.
    pub filename: Option<String>,
    /// Screenshot options.
    pub options: ScreenshotOptions,
}

/// Requests a screenshot with an auto-generated filename.
///
/// The screenshot will be saved as `screenshot_NNNN.png` in the current directory,
/// where NNNN is an auto-incrementing number.
///
/// This function can be called while `show()` is running.
/// The screenshot will be captured on the next frame.
///
/// # Example
///
/// ```no_run
/// use polyscope_rs::*;
///
/// init().unwrap();
/// // ... register structures ...
///
/// // Request a screenshot (will be saved when show() runs)
/// screenshot();
///
/// show();
/// ```
pub fn screenshot() {
    screenshot_with_options(ScreenshotOptions::default());
}

/// Requests a screenshot with custom options.
///
/// # Arguments
/// * `options` - Screenshot options (e.g., transparent background)
pub fn screenshot_with_options(options: ScreenshotOptions) {
    if let Ok(mut guard) = SCREENSHOT_REQUEST.lock() {
        *guard = Some(ScreenshotRequest {
            filename: None,
            options,
        });
    }
}

/// Requests a screenshot to be saved to a specific file.
///
/// # Arguments
/// * `filename` - The filename to save to (supports .png and .jpg)
///
/// # Example
///
/// ```no_run
/// use polyscope_rs::*;
///
/// init().unwrap();
/// // ... register structures ...
///
/// screenshot_to_file("my_scene.png");
/// show();
/// ```
pub fn screenshot_to_file(filename: impl Into<String>) {
    screenshot_to_file_with_options(filename, ScreenshotOptions::default());
}

/// Requests a screenshot to be saved to a specific file with custom options.
///
/// # Arguments
/// * `filename` - The filename to save to (supports .png and .jpg)
/// * `options` - Screenshot options
pub fn screenshot_to_file_with_options(filename: impl Into<String>, options: ScreenshotOptions) {
    if let Ok(mut guard) = SCREENSHOT_REQUEST.lock() {
        *guard = Some(ScreenshotRequest {
            filename: Some(filename.into()),
            options,
        });
    }
}

/// Takes and returns a pending screenshot request (for internal use by App).
pub(crate) fn take_screenshot_request() -> Option<ScreenshotRequest> {
    SCREENSHOT_REQUEST
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}
