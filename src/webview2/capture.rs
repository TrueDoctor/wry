use std::{cell::Cell, rc::Rc, sync::Arc};

use windows::{
  Foundation::AsyncActionCompletedHandler,
  System::DispatcherQueueController,
  Win32::{
    System::{Threading::GetCurrentThreadId, WinRT::RoInitialize},
    UI::WindowsAndMessaging::{
      DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
    },
  },
};
use windows_capture::{
  capture::{GraphicsCaptureApiError, GraphicsCaptureApiHandler},
  frame::{Frame, FrameBuffer},
  graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl},
  settings::Settings,
};

#[derive(Debug, Clone)]
pub(crate) struct OwnedFrame {
  pub data: Box<[u8]>,
  pub width: u32,
  pub height: u32,
}

impl<'a, 'b> TryFrom<&'a mut Frame<'b>> for OwnedFrame {
  fn try_from(value: &'a mut Frame<'b>) -> Result<Self, Self::Error> {
    Ok(OwnedFrame {
      data: Box::from(value.buffer()?.as_raw_nopadding_buffer()?.to_owned()),
      width: value.width(),
      height: value.height(),
    })
  }

  type Error = Box<dyn std::error::Error + Send + Sync>;
}

pub(crate) struct RecordingContext {
  frame: &'static std::sync::Mutex<Option<OwnedFrame>>,
}

impl RecordingContext {
  pub fn custom_start(
    settings: Settings<<RecordingContext as GraphicsCaptureApiHandler>::Flags>,
    controller: DispatcherQueueController,
  ) -> Result<(), GraphicsCaptureApiError<<RecordingContext as GraphicsCaptureApiHandler>::Error>>
  where
    Self: Send + 'static,
    <Self as GraphicsCaptureApiHandler>::Flags: Send,
  {
    // Get current thread ID
    let thread_id = unsafe { GetCurrentThreadId() };

    // Start capture
    let result = Arc::new(parking_lot::Mutex::new(None));
    let callback = Arc::new(parking_lot::Mutex::new(
      Self::new(settings.flags).map_err(GraphicsCaptureApiError::NewHandlerError)?,
    ));
    let mut capture = GraphicsCaptureApi::new(
      settings.item,
      callback,
      settings.cursor_capture,
      settings.draw_border,
      settings.color_format,
      thread_id,
      result.clone(),
    )
    .map_err(GraphicsCaptureApiError::GraphicsCaptureApiError)?;
    capture
      .start_capture()
      .map_err(GraphicsCaptureApiError::GraphicsCaptureApiError)?;

    // Message loop
    let mut message = MSG::default();
    unsafe {
      while GetMessageW(&mut message, None, 0, 0).as_bool() {
        TranslateMessage(&message);
        DispatchMessageW(&message);
      }
    }

    // Shutdown dispatcher queue
    let async_action = controller
      .ShutdownQueueAsync()
      .map_err(|_| GraphicsCaptureApiError::FailedToShutdownDispatcherQueue)?;
    async_action
      .SetCompleted(&AsyncActionCompletedHandler::new(
        move |_, _| -> Result<(), windows::core::Error> {
          unsafe { PostQuitMessage(0) };
          Ok(())
        },
      ))
      .map_err(|_| GraphicsCaptureApiError::FailedToSetDispatcherQueueCompletedHandler)?;

    // Final message loop
    let mut message = MSG::default();
    unsafe {
      while GetMessageW(&mut message, None, 0, 0).as_bool() {
        TranslateMessage(&message);
        DispatchMessageW(&message);
      }
    }

    // Stop capture
    capture.stop_capture();

    // Check handler result
    if let Some(e) = result.lock().take() {
      return Err(GraphicsCaptureApiError::FrameHandlerError(e));
    }

    Ok(())
  }
}

impl GraphicsCaptureApiHandler for RecordingContext {
  // The type of flags used to get the values from the settings.
  type Flags = &'static std::sync::Mutex<Option<OwnedFrame>>;

  // The type of error that can occur during capture, the error will be returned from `CaptureControl` and `start` functions.
  type Error = Box<dyn std::error::Error + Send + Sync>;

  // Function that will be called to create the struct. The flags can be passed from settings.
  fn new(frame_storage: Self::Flags) -> Result<Self, Self::Error> {
    println!("Starting frame capture");

    Ok(Self {
      frame: frame_storage,
    })
  }

  // Called every time a new frame is available.
  fn on_frame_arrived(
    &mut self,
    frame: &mut Frame,
    _capture_control: InternalCaptureControl,
  ) -> Result<(), Self::Error> {
    println!("got frame");
    dbg!(frame
      .buffer()
      .unwrap()
      .as_raw_nopadding_buffer()
      .unwrap()
      .iter()
      .max());
    *self.frame.lock().map_err(|_| "mutex poisoned")? = Some(frame.try_into()?);

    Ok(())
  }

  // Optional handler called when the capture item (usually a window) closes.
  fn on_closed(&mut self) -> Result<(), Self::Error> {
    println!("Capture Session Closed");

    Ok(())
  }
}
